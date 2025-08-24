use crate::auth::{handle_login, handle_logout};
use crate::list::handle_list;
use crate::manage::handle_manage;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::{Confirm, Text};
use std::env;
use std::path::Path;
use std::process::ExitCode;
use tokio::process::Command;
use wayclip_core::control::DaemonManager;
use wayclip_core::{
    Collect, PullClipsArgs, WAYCLIP_TRIGGER_PATH, api, delete_file, gather_clip_data,
    rename_all_entries, settings::Settings,
};

pub mod auth;
pub mod list;
pub mod manage;

#[derive(Parser)]
#[command(
    name = "wayclip",
    version,
    about = "An instant clipping tool with cloud sync, built on PipeWire and GStreamer."
)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Daemon {
        #[command(subcommand)]
        action: DaemonCommand,
    },
    Save,
    List {
        #[arg(short = 't', long = "timestamp")]
        timestamp: bool,
        #[arg(short = 'l', long = "length")]
        length: bool,
        #[arg(short = 'r', long = "reverse")]
        reverse: bool,
        #[arg(short = 's', long = "size")]
        size: bool,
        #[arg(short = 'e', long = "extra")]
        extra: bool,
    },
    Manage,
    Config {
        #[arg(short = 'e', long = "editor")]
        editor: Option<String>,
    },
    View {
        name: String,
        #[arg(short = 'p', long = "player")]
        player: Option<String>,
    },
    Delete {
        name: String,
    },
    Rename {
        name: String,
    },
    Edit {
        name: String,
        start_time: String,
        end_time: String,
        #[arg(default_value_t = false)]
        disable_audio: bool,
    },
    Login,
    Logout,
    Me,
    Share {
        #[arg(help = "Name of the clip to share")]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum DaemonCommand {
    Start,
    Stop,
    Restart,
    Status,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.debug {
        println!("{}", "Debug mode is ON".yellow());
    }

    match &cli.command {
        Commands::Login => handle_login().await?,
        Commands::Logout => handle_logout().await?,
        Commands::Me => handle_me().await?,
        Commands::Share { name } => handle_share(name).await?,
        Commands::Save => handle_save().await?,
        Commands::List { .. } => handle_list(&cli.command).await?,
        Commands::Manage => handle_manage().await?,
        Commands::Config { editor } => handle_config(editor.as_deref()).await?,
        Commands::View { name, player } => handle_view(name, player.as_deref()).await?,
        Commands::Rename { name } => handle_rename(name).await?,
        Commands::Delete { name } => handle_delete(name).await?,
        Commands::Edit { .. } => println!("Editing clip..."),
        Commands::Daemon { action } => {
            let manager = DaemonManager::new();
            match action {
                DaemonCommand::Start => manager.start().await?,
                DaemonCommand::Stop => manager.stop().await?,
                DaemonCommand::Restart => manager.restart().await?,
                DaemonCommand::Status => {
                    manager.status().await?;
                }
            }
        }
    }

    Ok(())
}

async fn handle_me() -> Result<()> {
    match api::get_current_user().await {
        Ok(profile) => {
            let usage_gb = profile.storage_used as f64 / 1_073_741_824.0;
            let limit_gb = profile.storage_limit as f64 / 1_073_741_824.0;
            let percentage = if profile.storage_limit > 0 {
                (usage_gb / limit_gb) * 100.0
            } else {
                0.0
            };

            println!("{}", "┌─ Your Profile ─────────".bold());
            println!("│ {} {}", "Username:".cyan(), profile.user.username);
            println!(
                "│ {} {}",
                "Tier:".cyan(),
                format!("{:?}", profile.user.tier).green()
            );
            println!("│ {} {}", "Hosted Clips:".cyan(), profile.clip_count);
            println!(
                "│ {} {:.2} GB / {:.2} GB ({:.1}%)",
                "Storage:".cyan(),
                usage_gb,
                limit_gb,
                percentage
            );
            println!("└────────────────────────");
        }
        Err(api::ApiClientError::Unauthorized) => {
            bail!("You are not logged in. Please run `wayclip login` first.");
        }
        Err(e) => {
            bail!("Failed to fetch profile: {}", e);
        }
    }
    Ok(())
}

async fn handle_share(clip_name: &str) -> Result<()> {
    println!("{}", "○ Preparing to share...".cyan());

    let profile = api::get_current_user()
        .await
        .context("Could not get user profile. Are you logged in?")?;
    let settings = Settings::load().await?;
    let clips_path = Settings::home_path().join(&settings.save_path_from_home_string);

    let clip_path = if clip_name.ends_with(".mp4") {
        clips_path.join(clip_name)
    } else {
        clips_path.join(format!("{}.mp4", clip_name))
    };

    if !clip_path.exists() {
        bail!("Clip '{}' not found locally.", clip_name);
    }

    let file_size = tokio::fs::metadata(&clip_path).await?.len() as i64;
    let available_storage = profile.storage_limit - profile.storage_used;

    if file_size > available_storage {
        bail!(
            "Upload rejected: File size ({:.2} MB) exceeds your available storage ({:.2} MB).",
            file_size as f64 / 1_048_576.0,
            available_storage as f64 / 1_048_576.0
        );
    }

    println!(
        "{}",
        "◌ Uploading clip... (this may take a moment)".yellow()
    );

    let client = api::get_api_client().await?;
    match api::share_clip(&client, &clip_path).await {
        Ok(url) => {
            println!("{}", "✔ Clip shared successfully!".green().bold());
            println!("  Public URL: {}", url.underline());
        }
        Err(api::ApiClientError::Unauthorized) => {
            bail!("You must be logged in to share clips. Please run `wayclip login`.");
        }
        Err(e) => {
            bail!("Failed to share clip: {}", e);
        }
    }
    Ok(())
}

async fn handle_save() -> Result<()> {
    let mut trigger_command = Command::new(WAYCLIP_TRIGGER_PATH);
    let status = trigger_command
        .status()
        .await
        .context("Failed to execute the trigger process. Is the daemon running?")?;
    if status.success() {
        println!("{}", "✔ Trigger process finished successfully.".green());
    } else {
        bail!("Trigger process failed with status: {}", status);
    }
    Ok(())
}

async fn handle_config(editor: Option<&str>) -> Result<()> {
    let editor_name = editor
        .map(String::from)
        .or_else(|| env::var("VISUAL").ok())
        .or_else(|| env::var("EDITOR").ok());
    let mut command = match editor_name {
        Some(editor) => {
            println!("Using editor: {}", &editor);
            let mut parts = editor.split_whitespace();
            let mut cmd = Command::new(parts.next().unwrap_or("nano"));
            cmd.args(parts);
            cmd
        }
        None => {
            println!("VISUAL and EDITOR not set, falling back to nano.");
            Command::new("nano")
        }
    };
    command.arg(
        Settings::config_path()
            .join("wayclip")
            .join("settings.json"),
    );
    let status = command.status().await.context("Failed to open editor")?;
    if !status.success() {
        bail!("Editor process failed with status: {}", status);
    }
    Ok(())
}

async fn handle_view(name: &str, player: Option<&str>) -> Result<()> {
    let settings = Settings::load().await?;
    let clips_path = Settings::home_path().join(&settings.save_path_from_home_string);
    let clip_file = clips_path.join(name);
    let player_name = player.unwrap_or("mpv");
    println!("⏵ Launching '{}' with {}...", name.cyan(), player_name);
    let mut parts = player_name.split_whitespace();
    let mut command = Command::new(parts.next().unwrap_or("mpv"));
    command.args(parts);
    command.arg(clip_file);
    let mut child = command
        .spawn()
        .context(format!("Failed to launch media player '{player_name}'"))?;

    let _ = child.wait().await;
    Ok(())
}

async fn handle_rename(name: &str) -> Result<()> {
    let clips = gather_clip_data(
        Collect::All,
        PullClipsArgs {
            page: 1,
            page_size: 999,
            search_query: Some(name.to_string()),
        },
    )
    .await?
    .clips;

    let clip_to_rename = clips.first().context(format!("Clip '{name}' not found."))?;

    let new_name_stem = Text::new("Enter new name (without extension):")
        .with_initial_value(&clip_to_rename.name)
        .prompt()?;

    if new_name_stem.is_empty() || new_name_stem == clip_to_rename.name {
        println!("{}", "Rename cancelled.".yellow());
        return Ok(());
    }

    let extension = Path::new(&clip_to_rename.path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");
    let new_full_name = format!("{}.{}", new_name_stem, extension);

    match rename_all_entries(&clip_to_rename.path, &new_full_name).await {
        Ok(_) => println!("{}", format!("✔ Renamed to '{}'", new_full_name).green()),
        Err(e) => bail!("Failed to rename: {}", e),
    }
    Ok(())
}

async fn handle_delete(name: &str) -> Result<()> {
    let clips = gather_clip_data(
        Collect::All,
        PullClipsArgs {
            page: 1,
            page_size: 999,
            search_query: Some(name.to_string()),
        },
    )
    .await?
    .clips;

    let clip_to_delete = clips.first().context(format!("Clip '{name}' not found."))?;

    let hosted_clips = api::get_hosted_clips_index().await.unwrap_or_default();
    let clip_filename = Path::new(&clip_to_delete.path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let hosted_info = hosted_clips.iter().find(|c| c.file_name == clip_filename);

    println!("Preparing to delete '{}'.", name.cyan());

    if let Some(hosted) = hosted_info {
        let confirmed = Confirm::new("This clip is hosted on the server. Delete the server copy?")
            .with_default(true)
            .prompt()?;
        if confirmed {
            let client = api::get_api_client().await?;
            api::delete_clip(&client, hosted.id).await?;
            println!("{}", "✔ Server copy deleted.".green());
        }
    }

    let confirmed_local = Confirm::new("Delete the local file? This cannot be undone.")
        .with_default(false)
        .prompt()?;
    if confirmed_local {
        delete_file(&clip_to_delete.path)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        println!("{}", "✔ Local file deleted.".green());
    }

    Ok(())
}
