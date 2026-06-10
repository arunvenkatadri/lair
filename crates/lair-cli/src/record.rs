//! `lair record` — run the project and capture its MCAP log.
//!
//! Recording is intrinsic to a LAIR app: the runtime logs every message to MCAP
//! while it runs. `lair record` runs the project like `lair run`, then locates the
//! freshest `.mcap` the run produced and (optionally) copies it to a chosen path,
//! so the recording is easy to find and hand off to `lair replay`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

/// Runs the current project and reports where its MCAP recording landed.
pub fn record(release: bool, output: Option<PathBuf>) -> Result<()> {
    let started = SystemTime::now();

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run");
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().context("failed to launch `cargo run`")?;
    if !status.success() {
        anyhow::bail!("project exited with status {status}");
    }

    // Find the newest .mcap produced at or after this run started.
    match newest_mcap_since(Path::new("."), started)? {
        Some(log) => {
            println!("Recorded MCAP log: {}", log.display());
            if let Some(dest) = output {
                std::fs::copy(&log, &dest).with_context(|| {
                    format!("failed to copy {} to {}", log.display(), dest.display())
                })?;
                println!("Copied to: {}", dest.display());
            }
            println!("Inspect it with: lair replay {}", log.display());
        }
        None => {
            println!(
                "No .mcap log found under {}. Ensure the app configures MCAP logging \
                 (see the example's `basic_copper_setup`).",
                std::env::current_dir()?.display()
            );
        }
    }

    Ok(())
}

/// Walks `root` (a few levels deep) for `.mcap` files modified at/after `since`,
/// returning the most recently modified one.
fn newest_mcap_since(root: &Path, since: SystemTime) -> Result<Option<PathBuf>> {
    let mut best: Option<(SystemTime, PathBuf)> = None;
    visit(root, 0, &mut |path, modified| {
        if path.extension().is_some_and(|e| e == "mcap") && modified >= since {
            if best.as_ref().is_none_or(|(t, _)| modified > *t) {
                best = Some((modified, path.to_path_buf()));
            }
        }
    })?;
    Ok(best.map(|(_, p)| p))
}

/// Recursively visits files up to a bounded depth, skipping common noise dirs.
fn visit(
    dir: &Path,
    depth: usize,
    on_file: &mut dyn FnMut(&Path, SystemTime),
) -> Result<()> {
    if depth > 4 {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            let name = entry.file_name();
            if matches!(name.to_str(), Some(".git") | Some("target") | Some("node_modules")) {
                continue;
            }
            visit(&path, depth + 1, on_file)?;
        } else if let Ok(modified) = meta.modified() {
            on_file(&path, modified);
        }
    }
    Ok(())
}
