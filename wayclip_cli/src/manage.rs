use crate::{handle_share, handle_view};
use anyhow::Result;
use arboard::Clipboard;
use chrono::Utc;
use colored::*;
use inquire::{Confirm, InquireError, Select, Text};
use std::collections::HashMap;
use std::path::Path;
use std::thread;
use std::time::Duration;
use wayclip_core::{
    api, delete_file, gather_unified_clips, models::UnifiedClipData, rename_all_entries,
    update_liked,
};

pub async fn handle_manage() -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    let settings = wayclip_core::settings::Settings::load().await?;

    'main_loop: loop {
        let mut all_clips = gather_unified_clips().await?;

        if all_clips.is_empty() {
            println!("{}", "No clips found locally or on the server.".yellow());
            return Ok(());
        }

        let sort_options = vec![
            "Date (Newest First)",
            "Name (A-Z)",
            "Liked First",
            "Hosted First",
            "[Quit]",
        ];
        let sort_choice = match Select::new("Filter / Sort clips:", sort_options).prompt() {
            Ok(choice) => choice,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => break,
            Err(e) => return Err(e.into()),
        };

        match sort_choice {
            "Date (Newest First)" => all_clips.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            "Name (A-Z)" => {
                all_clips.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }
            "Liked First" => all_clips.sort_by(|a, b| {
                b.local_data
                    .as_ref()
                    .map_or(false, |d| d.liked)
                    .cmp(&a.local_data.as_ref().map_or(false, |d| d.liked))
                    .then(b.created_at.cmp(&a.created_at))
            }),
            "Hosted First" => all_clips.sort_by(|a, b| {
                b.is_hosted
                    .cmp(&a.is_hosted)
                    .then(b.created_at.cmp(&a.created_at))
            }),
            _ => break,
        }

        let now = Utc::now();
        let mut clip_options: Vec<String> = Vec::new();
        let mut clip_map: HashMap<String, &UnifiedClipData> = HashMap::new();

        for clip in &all_clips {
            let clip_age = now.signed_duration_since(clip.created_at.with_timezone(&Utc));
            let display_name = format!(
                "{} {} {}{}{}",
                if clip.local_path.is_some() {
                    "💻"
                } else {
                    "  "
                },
                if clip.is_hosted { "🌐" } else { "  " },
                clip.local_data
                    .as_ref()
                    .map_or("".normal(), |d| if d.liked {
                        "♥ ".red()
                    } else {
                        "".normal()
                    }),
                clip.name,
                if clip_age < chrono::Duration::hours(24) {
                    " [NEW]".yellow()
                } else {
                    "".normal()
                }
            );
            clip_options.push(display_name.clone());
            clip_map.insert(display_name, clip);
        }

        clip_options.insert(0, "[Quit]".to_string());
        let selected_display_name = match Select::new("Select a clip to manage:", clip_options)
            .with_page_size(20)
            .prompt()
        {
            Ok(choice) => choice,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => break,
            Err(e) => return Err(e.into()),
        };

        if selected_display_name == "[Quit]" {
            break 'main_loop;
        }
        let selected_clip = match clip_map.get(&selected_display_name) {
            Some(clip) => *clip,
            None => {
                println!("{}", "Could not find the clip.".red());
                continue;
            }
        };

        let mut options = Vec::new();
        if selected_clip.is_hosted {
            options.push("🔗 Open URL");
            options.push("📋 Copy URL");
        }
        if selected_clip.local_path.is_some() {
            options.push("▷ View Local File");
            options.push("✎ Rename");
            options.push("⎘ Copy Name");
            if selected_clip.local_data.as_ref().map_or(false, |d| d.liked) {
                options.push("♡ Unlike");
            } else {
                options.push("♥ Like");
            }
        }
        if !selected_clip.is_hosted && selected_clip.local_path.is_some() {
            options.push("↗ Share");
        }
        if selected_clip.is_hosted {
            options.push("🗑️ Delete Server Copy");
        }
        if selected_clip.local_path.is_some() {
            options.push("🗑️ Delete Local File");
        }
        options.push("← Back to List");

        let action = match Select::new(
            &format!("Action for '{}':", selected_clip.name.cyan()),
            options,
        )
        .prompt()
        {
            Ok(choice) => choice,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => continue,
            Err(e) => return Err(e.into()),
        };

        match action {
            "▷ View Local File" => handle_view(&selected_clip.full_filename, None).await?,
            "🔗 Open URL" => {
                let public_url = format!(
                    "{}/clip/{}",
                    settings.api_url,
                    selected_clip.hosted_id.unwrap()
                );
                opener::open(public_url)?;
            }
            "✎ Rename" => {
                let local_path = selected_clip.local_path.as_ref().unwrap();
                let new_name_stem = Text::new("Enter new name (without extension):")
                    .with_initial_value(&selected_clip.name)
                    .prompt()?;
                if new_name_stem.is_empty() || new_name_stem == selected_clip.name {
                    println!("{}", "Rename cancelled.".yellow());
                    continue;
                }
                let extension = Path::new(local_path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("mp4");
                let new_full_name = format!("{new_name_stem}.{extension}");
                match rename_all_entries(local_path, &new_full_name).await {
                    Ok(_) => println!("{}", format!("✔ Renamed to '{new_full_name}'").green()),
                    Err(e) => println!("{}", format!("✗ Failed to rename: {e}").red()),
                }
                continue 'main_loop;
            }
            "⎘ Copy Name" => {
                clipboard.set_text(&selected_clip.name)?;
                thread::sleep(Duration::from_millis(100));
                println!("{}", "✔ Name copied to clipboard!".green());
            }
            "♥ Like" | "♡ Unlike" => {
                let local_data = selected_clip.local_data.as_ref().unwrap();
                match update_liked(&selected_clip.full_filename, !local_data.liked).await {
                    Ok(_) => println!("{}", "✔ Liked status updated!".green()),
                    Err(e) => println!("{}", format!("✗ Failed to update liked status: {e}").red()),
                }
                continue 'main_loop;
            }
            "↗ Share" => {
                if let Err(e) = handle_share(&selected_clip.name).await {
                    println!("{} {}", "✗ Share failed:".red(), e);
                }
                continue 'main_loop;
            }
            "📋 Copy URL" => {
                if let Some(id) = selected_clip.hosted_id {
                    let public_url = format!("{}/clip/{}", settings.api_url, id);
                    clipboard.set_text(public_url)?;
                    thread::sleep(Duration::from_millis(100));
                    println!("{}", "✔ Public URL copied to clipboard!".green());
                }
            }
            "🗑️ Delete Server Copy" => {
                let confirmed =
                    Confirm::new("Are you sure? This will delete the clip from the server.")
                        .with_default(false)
                        .prompt()?;
                if confirmed {
                    let client = api::get_api_client().await?;
                    api::delete_clip(&client, selected_clip.hosted_id.unwrap()).await?;
                    println!("{}", "✔ Server copy deleted.".green());
                }
                continue 'main_loop;
            }
            "🗑️ Delete Local File" => {
                let confirmed = Confirm::new("Are you sure? This will delete the local file.")
                    .with_default(false)
                    .prompt()?;
                if confirmed {
                    delete_file(selected_clip.local_path.as_ref().unwrap())
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    println!("{}", "✔ Local file deleted.".green());
                }
                continue 'main_loop;
            }
            _ => continue,
        }
        println!();
    }
    Ok(())
}
