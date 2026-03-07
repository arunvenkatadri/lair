use anyhow::Result;
use std::process::Command;

pub fn run_doctor() -> Result<()> {
    println!("LAIR Doctor — checking system requirements\n");

    check_cargo();
    check_rustc();

    println!("\nDone.");
    Ok(())
}

fn check_cargo() {
    print!("  cargo ... ");
    match Command::new("cargo").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("OK ({})", version.trim());
        }
        _ => {
            println!("NOT FOUND");
            println!("    Install Rust via https://rustup.rs");
        }
    }
}

fn check_rustc() {
    print!("  rustc ... ");
    match Command::new("rustc").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            let version_str = version_str.trim();

            // Parse version: "rustc X.Y.Z ..."
            let ok = parse_rustc_version(version_str)
                .map(|(major, minor)| major > 1 || (major == 1 && minor >= 75))
                .unwrap_or(false);

            if ok {
                println!("OK ({})", version_str);
            } else {
                println!("TOO OLD ({})", version_str);
                println!("    LAIR requires rustc >= 1.75. Run: rustup update");
            }
        }
        _ => {
            println!("NOT FOUND");
            println!("    Install Rust via https://rustup.rs");
        }
    }
}

fn parse_rustc_version(s: &str) -> Option<(u32, u32)> {
    // "rustc 1.75.0 (hash date)"
    let version_part = s.strip_prefix("rustc ")?.split_whitespace().next()?;
    let mut parts = version_part.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_string() {
        assert_eq!(
            parse_rustc_version("rustc 1.82.0 (f6e511eec 2024-10-15)"),
            Some((1, 82))
        );
        assert_eq!(
            parse_rustc_version("rustc 1.75.0"),
            Some((1, 75))
        );
    }
}
