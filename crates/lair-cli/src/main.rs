mod doctor;
mod new_project;
mod record;
mod replay;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// LAIR — the Rust engine for autonomous robots.
#[derive(Parser)]
#[command(name = "lair", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new LAIR project
    New {
        /// Project name
        name: String,
    },
    /// Build the current LAIR project
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Run the current LAIR project
    Run {
        /// Run in release mode
        #[arg(long)]
        release: bool,
    },
    /// Run the project and capture its MCAP recording
    Record {
        /// Run in release mode
        #[arg(long)]
        release: bool,
        /// Copy the produced log to this path
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
    /// Inspect a recorded MCAP log
    Replay {
        /// Path to the .mcap log file
        file: PathBuf,
        /// Also print the first N message records
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
    },
    /// Check system requirements
    Doctor,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            new_project::create_project(&name)?;
        }
        Commands::Build { release } => {
            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("build");
            if release {
                cmd.arg("--release");
            }
            let status = cmd.status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Run { release } => {
            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("run");
            if release {
                cmd.arg("--release");
            }
            let status = cmd.status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Record { release, output } => {
            record::record(release, output)?;
        }
        Commands::Replay { file, limit } => {
            replay::replay_file(&file, limit)?;
        }
        Commands::Doctor => {
            doctor::run_doctor()?;
        }
    }

    Ok(())
}
