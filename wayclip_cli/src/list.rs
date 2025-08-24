use crate::Commands;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, ContentArrangement, Table};
use wayclip_core::{Collect, PullClipsArgs, gather_clip_data};

pub async fn handle_list(command: &Commands) -> Result<()> {
    let Commands::List {
        timestamp,
        length,
        reverse,
        size,
        extra,
    } = command
    else {
        unreachable!()
    };

    let mut clips = gather_clip_data(
        Collect::All,
        PullClipsArgs {
            page: 1,
            page_size: 100,
            search_query: None,
        },
    )
    .await
    .context("Could not list clips")?
    .clips;

    if clips.is_empty() {
        println!("{}", "No clips found.".yellow());
        return Ok(());
    }

    if *reverse {
        clips.reverse();
    } else {
        clips.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    println!("Found {} clips:", clips.len());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let mut headers = vec![Cell::new("Clip Name").add_attribute(comfy_table::Attribute::Bold)];
    if *timestamp {
        headers.push(Cell::new("Date Created").add_attribute(comfy_table::Attribute::Bold));
    }
    if *size {
        headers.push(Cell::new("Size").add_attribute(comfy_table::Attribute::Bold));
    }
    if *length {
        headers.push(Cell::new("Duration").add_attribute(comfy_table::Attribute::Bold));
    }
    if *extra {
        headers.push(Cell::new("Metadata").add_attribute(comfy_table::Attribute::Bold));
    }
    table.set_header(headers);

    let now = Utc::now();

    for clip in clips {
        let mut row = Vec::new();

        let clip_age = now.signed_duration_since(clip.created_at);
        let display_name = if clip_age < chrono::Duration::hours(24) {
            format!("{} {}", clip.name, "[NEW]".yellow())
        } else {
            clip.name.clone()
        };
        row.push(Cell::new(display_name));

        if *timestamp {
            row.push(Cell::new(clip.created_at.format("%Y-%m-%d %H:%M")));
        }
        if *size {
            row.push(Cell::new(format!(
                "{:.2} MB",
                clip.size as f64 / 1_048_576.0
            )));
        }
        if *length {
            row.push(Cell::new(format!("{:.2}s", clip.length)));
        }
        if *extra {
            let mut meta = Vec::new();
            if clip.liked {
                meta.push("♥".to_string());
            }
            if !clip.tags.is_empty() {
                meta.push(format!(
                    "[{}]",
                    clip.tags
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            row.push(Cell::new(meta.join(" ")));
        }
        table.add_row(row);
    }

    println!("{table}");
    Ok(())
}
