use crate::auth::{handle_login, handle_logout};
use crate::list::handle_list;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::{Select, Text};
use std::env;
use std::process::ExitCode;
use tokio::process::Command;
use wayclip_core::control::DaemonManager;
use wayclip_core::{
    Collect, PullClipsArgs, WAYCLIP_TRIGGER_PATH, api, gather_clip_data, settings::Settings,
};

pub mod auth;
pub mod list;

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
        Commands::Rename { name } => println!("Renaming clip: {name}"),
        Commands::Delete { name } => println!("Deleting clip: {name}"),
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

async fn handle_manage() -> Result<()> {
    loop {
        let mut clips = gather_clip_data(
            Collect::All,
            PullClipsArgs {
                page: 1,
                page_size: 100,
                search_query: None,
            },
        )
        .await?
        .clips;

        if clips.is_empty() {
            println!("{}", "No clips found to manage.".yellow());
            return Ok(());
        }

        clips.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let mut clip_options: Vec<String> = clips.iter().map(|c| c.name.clone()).collect();
        clip_options.insert(0, "[Quit]".to_string());

        let selected_clip_name = Select::new("Select a clip to manage:", clip_options).prompt()?;

        if selected_clip_name == "[Quit]" {
            break;
        }

        let options = vec![
            "▷ View",
            "✎ Rename",
            "✗ Delete",
            "⎘ Copy Name",
            "← Back to List",
        ];
        let action = Select::new(
            &format!("Action for '{}':", selected_clip_name.cyan()),
            options,
        )
        .prompt()?;

        match action {
            "▷ View" => {
                handle_view(&selected_clip_name, None).await?;
            }
            "✎ Rename" => {
                let new_name = Text::new("Enter new name:").prompt()?;
                println!("Renaming '{selected_clip_name}' to '{new_name}' (Not yet implemented)",);
            }
            "✗ Delete" => {
                println!("Deleting '{selected_clip_name}' (Not yet implemented)");
            }
            "⎘ Copy Name" => {
                let mut clipboard = arboard::Clipboard::new()?;
                clipboard.set_text(&selected_clip_name)?;
                println!("{}", "✔ Name copied to clipboard!".green());
            }
            _ => continue,
        }
        println!();
    }
    Ok(())
}

async fn handle_me() -> Result<()> {
    match api::get_current_user().await {
        Ok(user) => {
            println!("{}", "┌─ Your Profile ─────────".bold());
            println!("│ {} {}", "Username:".cyan(), user.username);
            println!("│ {} {}", "User ID:".cyan(), user.id);
            println!(
                "│ {} {}",
                "Tier:".cyan(),
                format!("{:?}", user.tier).green()
            );
            println!(
                "│ {} {}",
                "Member Since:".cyan(),
                user.created_at.format("%Y-%m-%d")
            );
            println!("{}", "└────────────────────────".bold());
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
    println!("Attempting to share clip: '{clip_name}'...");
    let client = api::get_api_client().await?;
    match api::share_clip(&client, clip_name).await {
        Ok(_) => println!("{}", "✔ Clip shared successfully! (Placeholder)".green()),
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
    let status = command
        .status()
        .await
        .context(format!("Failed to launch media player '{player_name}'"))?;
    if !status.success() {
        bail!("Media player process failed with status: {}", status);
    }
    Ok(())
}
