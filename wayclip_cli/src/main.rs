use clap::{Parser, Subcommand};
use std::env;
use std::process::ExitCode;
use tokio::process::Command;
use wayclip_core::{Settings, WAYCLIP_TRIGGER_PATH, gather_clip_data};

#[derive(Parser)]
#[command(
    name = "wayclip",
    version,
    about = "An instant clipping tool built on top of PipeWire and GStreamer using Rust."
)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Daemon {
        #[command(subcommand)]
        action: DaemonCommand,
    },
    Status,
    Save,
    List {
        #[arg(
            short = 't',
            long = "timestamp",
            help = "Sort by creation time (newest first)"
        )]
        timestamp: bool,
        #[arg(
            short = 'l',
            long = "length",
            help = "Sort by clip duration (longest first)"
        )]
        length: bool,
        #[arg(short = 'r', long = "reverse", help = "Reverse sort order")]
        reverse: bool,
        #[arg(short = 's', long = "size", help = "Sort by file size (largest first)")]
        size: bool,
        #[arg(
            short = 'e',
            long = "extra",
            help = "Show all metadata (tags, liked, etc.)"
        )]
        extra: bool,
    },
    Config {
        #[arg(short = 'e', long = "editor", help = "Use a preferred editor")]
        editor: Option<String>,
    },
    View {
        // Sometime in future add autocompletion and hints
        #[arg(help = "Name of the clip to view")]
        name: String,
        #[arg(short = 'p', long = "player", help = "Use a preferred media player")]
        player: Option<String>,
    },
    Delete {
        // Sometime in future add autocompletion and hints
        #[arg(help = "Name of the clip to delete")]
        name: String,
    },
    Rename {
        // Sometime in future add autocompletion and hints
        #[arg(help = "Name of the clip to rename")]
        name: String,
    },
    Edit {
        // Sometime in future add autocompletion and hints
        #[arg(help = "Name of clip to edit (trimming)")]
        name: String,
        #[arg(help = "Start time in seconds or hh:mm:ss")]
        start_time: String,
        #[arg(help = "End time in seconds or hh:mm:ss")]
        end_time: String,
        #[arg(help = "Disable audio (true/false)", default_value_t = false)]
        disable_audio: bool,
    },
}

#[derive(Subcommand)]
enum DaemonCommand {
    Start,
    Stop,
    Restart,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = Settings::home_path().join(Settings::load().save_path_from_home_string);

    if cli.debug {
        println!("Debug mode is ON");
    }

    match &cli.command {
        Commands::Save => {
            let mut trigger_command = Command::new(WAYCLIP_TRIGGER_PATH);

            match trigger_command.status().await {
                Ok(status) => {
                    if status.success() {
                        println!("Trigger process finished successfully.");
                    } else {
                        eprintln!("Trigger process failed with status: {status}");
                        return ExitCode::FAILURE;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute the trigger process.");
                    eprintln!("           Reason: {e}",);
                    eprintln!(
                        "           Please check if the path is correct and the file is executable."
                    );
                    return ExitCode::FAILURE;
                }
            }
        }

        Commands::List {
            timestamp,
            length,
            reverse,
            size,
            extra,
        } => {
            let mut clips = match gather_clip_data(wayclip_core::Collect::All).await {
                Ok(clips) => clips,
                Err(e) => {
                    eprintln!("Error: Could not list clips: {e:?}");
                    return ExitCode::FAILURE;
                }
            };

            clips.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            if *reverse {
                clips.reverse();
            }

            if clips.is_empty() {
                println!("No clips found.");
                return ExitCode::SUCCESS;
            }

            println!("Found {} clips:", clips.len());

            let max_name_len = clips.iter().map(|c| c.name.len()).max().unwrap_or(20);

            for clip in clips {
                let mut output_parts = Vec::new();

                output_parts.push(format!("{:<width$}", clip.name, width = max_name_len));
                if *timestamp {
                    output_parts.push(clip.created_at.format("%Y-%m-%d %H:%M").to_string());
                }
                if *size {
                    output_parts.push(format!("{:>7.2} MB", clip.size as f64 / 1_048_576.0));
                }
                if *length {
                    output_parts.push(format!("{:>6.2}s", clip.length));
                }
                if *extra {
                    let mut extra_details = Vec::new();
                    if clip.liked {
                        extra_details.push("♥ Liked".to_string());
                    }
                    if !clip.tags.is_empty() {
                        let tags_str = clip
                            .tags
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        extra_details.push(format!("Tags: [{tags_str}]"));
                    }

                    if !extra_details.is_empty() {
                        output_parts.push(extra_details.join(" | "));
                    }
                }

                println!("{}", output_parts.join("   "));
            }
        }
        Commands::Config { editor } => {
            let editor_name = if let Some(user_editor) = editor {
                Ok(user_editor.clone())
            } else {
                env::var("VISUAL").or_else(|_| env::var("EDITOR"))
            };

            let mut command = match editor_name {
                Ok(editor_name) => {
                    println!("Using editor: {}", &editor_name);
                    let mut parts = editor_name.split_whitespace();
                    let mut cmd = Command::new(parts.next().unwrap());
                    cmd.args(parts);
                    cmd
                }
                Err(_) => {
                    println!("VISUAL and EDITOR not set, falling back to nano.");
                    Command::new("nano")
                }
            };

            command.arg(
                Settings::config_path()
                    .join("wayclip")
                    .join("settings.json"),
            );

            match command.status().await {
                Ok(status) => {
                    if status.success() {
                        println!("Opened config successfully.");
                    } else {
                        eprintln!("Process failed with status: {status}");
                        return ExitCode::FAILURE;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to open config.");
                    eprintln!("           Reason: {e}",);
                    eprintln!("           Please check if the path is correct.");
                    return ExitCode::FAILURE;
                }
            }
        }

        Commands::View { name, player } => {
            let player_name = player.clone().unwrap_or_else(|| String::from("mpv"));

            println!("Using player: {}", &player_name);

            let mut parts = player_name.split_whitespace();
            let mut command = Command::new(parts.next().unwrap());
            command.args(parts);

            command.arg(path.join(name));

            match command.status().await {
                Ok(status) if status.success() => {
                    println!("Viewing clip successfully.");
                }
                Ok(status) => {
                    eprintln!("Process failed with status: {status}");
                    return ExitCode::FAILURE;
                }
                Err(e) => {
                    eprintln!("Failed to view clip.");
                    eprintln!("           Reason: {e}");
                    eprintln!("           Please check if the name is correct.");
                    return ExitCode::FAILURE;
                }
            }
        }

        Commands::Rename { name } => println!("Renaming clip: {name}"),
        Commands::Delete { name } => println!("Deleting clip: {name}"),
        Commands::Edit {
            name,
            start_time,
            end_time,
            disable_audio,
        } => println!(
            "Editing clip {name} from {start_time} to {end_time}, disable audio: {disable_audio}"
        ),
        Commands::Status => println!("Status of Wayclip"),
        Commands::Daemon { action } => match action {
            DaemonCommand::Start => println!("Starting daemon"),
            DaemonCommand::Stop => println!("Stopping daemon"),
            DaemonCommand::Restart => println!("Restarting daemon"),
        },
    }

    ExitCode::SUCCESS
}
