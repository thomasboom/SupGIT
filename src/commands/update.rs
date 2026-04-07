use anyhow::{bail, Context, Result};
use log::debug;
use std::env;
use std::io::IsTerminal;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const UPDATE_CHECK_INTERVAL_SECS: u64 = 3 * 24 * 60 * 60;

fn get_current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn get_last_check_file() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|p| p.join("supgit").join("last_update_check"))
}

fn get_time_since_last_check() -> Option<Duration> {
    let path = get_last_check_file()?;
    let contents = std::fs::read_to_string(path).ok()?;
    let timestamp: u64 = contents.trim().parse().ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(Duration::from_secs(now.saturating_sub(timestamp)))
}

fn record_update_check() {
    if let Some(path) = get_last_check_file() {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        let _ = std::fs::write(&path, now);
    }
}

pub fn check_and_auto_update() {
    if env::var("SupGIT_SKIP_UPDATE_CHECK").is_ok() {
        return;
    }

    if !std::io::stdout().is_terminal() {
        return;
    }

    let handle = std::thread::spawn(|| {
        if let Some(elapsed) = get_time_since_last_check() {
            if elapsed.as_secs() < UPDATE_CHECK_INTERVAL_SECS {
                return;
            }
        }

        let update_check_result = (|| -> Result<()> {
            let output = Command::new("cargo")
                .args(["install", "supgit", "--dry-run"])
                .output()
                .context("failed to check for updates - is cargo installed?")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}{}", stdout, stderr);

                if combined.contains("Installing supgit") || combined.contains("Would install") {
                    println!("Update available for supgit");
                }
            }
            Ok(())
        })();

        if update_check_result.is_ok() {
            record_update_check();
        }

        if let Err(e) = update_check_result {
            debug!("update check failed: {:?}", e);
        }
    });

    // Allow up to 2 seconds for the update check to complete
    let _ = handle.join();
}

pub fn run_self_update(_target_version: Option<&str>) -> Result<()> {
    println!("Current version: v{}", get_current_version());
    println!("Updating via cargo...");

    let status = Command::new("cargo")
        .args(["install", "supgit", "--force"])
        .status()
        .context("Failed to run cargo install")?;

    if !status.success() {
        bail!("cargo install failed with exit code {:?}", status.code());
    }

    println!("✓ Update complete");
    Ok(())
}
