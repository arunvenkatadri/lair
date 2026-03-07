mod doctor;
mod new_project;

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
        Commands::Doctor => {
            doctor::run_doctor()?;
        }
    }

    Ok(())
}
