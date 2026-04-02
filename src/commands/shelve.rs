use std::process::Command as StdCommand;

use anyhow::{Context, Result, bail};
use dialoguer::{Confirm, Input, Select};

use crate::git::run_git_silent;

pub struct StashInfo {
    pub index: usize,
    pub message: String,
}

pub fn get_stashes() -> Result<Vec<StashInfo>> {
    let output = StdCommand::new("git")
        .args(["stash", "list"])
        .output()
        .context("running git stash list")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stashes: Vec<StashInfo> = Vec::new();

    for (index, line) in stdout.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let message = if line.starts_with("stash@{}: ") {
            let start = line.find(": ").map(|i| i + 2).unwrap_or(0);
            line[start..].to_string()
        } else {
            line.to_string()
        };

        stashes.push(StashInfo { index, message });
    }

    Ok(stashes)
}

pub fn create_stash(message: Option<&str>) -> Result<()> {
    let mut args = vec!["stash", "push"];
    if let Some(msg) = message
        && !msg.trim().is_empty()
    {
        args.push("-m");
        args.push(msg);
    }

    run_git_silent(&args)?;
    println!("✓ Stashed changes");
    Ok(())
}

pub fn apply_stash(index: usize, drop: bool) -> Result<()> {
    let stash_ref = format!("stash@{{{}}}", index);

    if drop {
        run_git_silent(&["stash", "drop", &stash_ref])?;
        println!("✓ Dropped stash@{{{}}}", index);
    } else {
        run_git_silent(&["stash", "apply", &stash_ref])?;
        println!("✓ Applied stash@{{{}}}", index);
    }
    Ok(())
}

pub fn unshelve_stash(index: usize) -> Result<()> {
    let stash_ref = format!("stash@{{{}}}", index);
    run_git_silent(&["stash", "pop", &stash_ref])?;
    println!("✓ Unshelved stash@{{{}}}", index);
    Ok(())
}

pub fn clear_stash() -> Result<()> {
    run_git_silent(&["stash", "clear"])?;
    println!("✓ Cleared all stashes");
    Ok(())
}

pub fn run_shelve_interactive(non_interactive: bool) -> Result<()> {
    let stashes = get_stashes()?;

    if non_interactive {
        if stashes.is_empty() {
            println!("No stashes found.");
            println!("Use 'supgit shelve' or 'supgit shelve save <message>' to create one.");
        } else {
            println!("Stashed changes:");
            for stash in &stashes {
                println!("  stash@{{{}}}: {}", stash.index, stash.message);
            }
        }
        return Ok(());
    }

    let mut options: Vec<String> = Vec::new();
    if !stashes.is_empty() {
        for stash in &stashes {
            options.push(format!("stash@{{{}}}: {}", stash.index, stash.message));
        }
        options.push("Apply and keep (git stash apply)".to_string());
        options.push("Apply and remove (git stash pop)".to_string());
    }
    options.push("Save current changes (stash push)".to_string());
    options.push("Clear all stashes".to_string());

    let selection = Select::new()
        .with_prompt("Select a stash action")
        .items(&options)
        .default(0)
        .interact()?;

    let has_stashes = !stashes.is_empty();
    let apply_keep_idx = if has_stashes { stashes.len() } else { 0 };
    let apply_pop_idx = apply_keep_idx + 1;
    let save_idx = apply_pop_idx + 1;
    let clear_idx = save_idx + 1;

    if has_stashes && selection < stashes.len() {
        let stash = &stashes[selection];
        let sub_options = vec!["Apply and keep", "Apply and remove (unshelve)", "Drop"];
        let sub_selection = Select::new()
            .with_prompt(format!("What to do with stash@{{{}}}", stash.index))
            .items(&sub_options)
            .default(0)
            .interact()?;

        match sub_selection {
            0 => apply_stash(stash.index, false)?,
            1 => unshelve_stash(stash.index)?,
            2 => {
                let confirmed = Confirm::new()
                    .with_prompt(format!("Drop stash@{{{}}}?", stash.index))
                    .default(false)
                    .interact()?;
                if confirmed {
                    apply_stash(stash.index, true)?;
                } else {
                    println!("Cancelled.");
                }
            }
            _ => {}
        }
    } else if (!has_stashes && selection == 0) || (has_stashes && selection == apply_keep_idx) {
        if stashes.is_empty() {
            bail!("no stashes to apply");
        }
        let selection = Select::new()
            .with_prompt("Select a stash to apply and keep")
            .items(
                &stashes
                    .iter()
                    .map(|s| format!("stash@{{{}}}: {}", s.index, s.message))
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()?;
        apply_stash(stashes[selection].index, false)?;
    } else if (!has_stashes && selection == 1) || (has_stashes && selection == apply_pop_idx) {
        if stashes.is_empty() {
            bail!("no stashes to unshelve");
        }
        let selection = Select::new()
            .with_prompt("Select a stash to apply and remove")
            .items(
                &stashes
                    .iter()
                    .map(|s| format!("stash@{{{}}}: {}", s.index, s.message))
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()?;
        unshelve_stash(stashes[selection].index)?;
    } else if (!has_stashes && selection == 1) || (has_stashes && selection == save_idx) {
        let message: String = Input::new()
            .with_prompt("Stash message (optional)")
            .interact()?;
        create_stash(if message.trim().is_empty() {
            None
        } else {
            Some(&message)
        })?;
    } else if (!has_stashes && selection == 2) || (has_stashes && selection == clear_idx) {
        if stashes.is_empty() {
            println!("No stashes to clear.");
            return Ok(());
        }
        let confirmed = Confirm::new()
            .with_prompt("Clear ALL stashes? This cannot be undone.")
            .default(false)
            .interact()?;
        if confirmed {
            clear_stash()?;
        } else {
            println!("Cancelled.");
        }
    }

    Ok(())
}
