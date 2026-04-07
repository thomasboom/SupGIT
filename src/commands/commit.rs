use std::process::Command as StdCommand;

use anyhow::{bail, Result};
use dialoguer::{Confirm, FuzzySelect, Input};

use crate::git::{run_git_quiet, run_git_silent};
use crate::status::{get_current_branch, get_repo_root, PorcelainStatus};

pub fn run_commit(
    message: Option<String>,
    all: bool,
    staged: bool,
    unstaged: bool,
    push: bool,
    amend: bool,
    no_verify: bool,
    non_interactive: bool,
) -> Result<()> {
    let is_interactive = message.is_none() && !all && !staged && !unstaged;

    let (all, staged, unstaged, commit_msg, push, custom_files) = if is_interactive {
        if non_interactive {
            bail!("commit requires --message in non-interactive mode");
        }

        let scope = FuzzySelect::new()
            .with_prompt("What would you like to commit?")
            .items(&[
                "Staged changes",
                "Unstaged changes",
                "All changes",
                "Custom",
            ])
            .default(0)
            .interact()?;

        let (all, staged, unstaged) = match scope {
            0 => (false, true, false),
            1 => (false, false, true),
            2 => (true, false, false),
            _ => (false, false, false),
        };

        let mut custom_files: Vec<String> = Vec::new();
        if scope == 3 {
            let status = PorcelainStatus::parse()?;
            let files: Vec<&str> = status.all_uncommitted_files();
            if files.is_empty() {
                println!("No files to commit.");
                return Ok(());
            }
            let files_owned: Vec<String> = files.iter().map(|s| s.to_string()).collect();
            let selected = dialoguer::MultiSelect::new()
                .with_prompt("Select files to stage")
                .items(&files_owned)
                .interact()?;

            if selected.is_empty() {
                println!("No files selected.");
                return Ok(());
            }

            for idx in selected {
                custom_files.push(files_owned[idx].clone());
            }
        }

        let msg: String = Input::new().with_prompt("Commit message").interact()?;
        let should_push = Confirm::new()
            .with_prompt("Push after committing?")
            .default(false)
            .interact()?;
        (all, staged, unstaged, msg, should_push, custom_files)
    } else {
        let msg = message.unwrap_or_default();
        (all, staged, unstaged, msg, push, Vec::new())
    };

    if commit_msg.trim().is_empty() {
        bail!("commit message cannot be empty");
    }

    if staged && (all || unstaged) {
        bail!("cannot combine --staged with --all or --unstaged");
    }

    if amend && !no_verify {
        let has_commits = StdCommand::new("git")
            .args(["log", "--oneline", "-n", "1"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if has_commits {
            eprintln!("Warning: amending a commit that may have been pushed can cause issues.");
            eprintln!("  Use --no-verify to skip this check if you're sure.");
            if non_interactive {
                eprintln!("(non-interactive mode, proceeding with amend)");
            } else {
                let confirm = Confirm::new()
                    .with_prompt("Continue with amend?")
                    .default(false)
                    .interact()?;
                if !confirm {
                    println!("Aborted.");
                    return Ok(());
                }
            }
        }
    }

    if all {
        run_git_silent(&["add", "-A"])?;
        println!("Staged all files");
    } else if unstaged {
        run_git_silent(&["add", "-u"])?;
        println!("Staged tracked files");
    } else if !custom_files.is_empty() {
        let repo_root = get_repo_root()?;
        let mut args = vec!["add".to_string()];
        args.extend(custom_files.iter().cloned());
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        crate::git::run_git_in_dir_silent(&args_refs, &repo_root)?;
        println!("Staged {} file(s)", custom_files.len());
    }

    print!("Committing");
    if amend {
        print!(" (amend)");
    }
    println!("...");

    let mut commit_args = vec!["commit"];
    if amend {
        commit_args.push("--amend");
    }
    if no_verify {
        commit_args.push("--no-verify");
    }
    commit_args.push("-m");
    commit_args.push(commit_msg.as_str());

    run_git_quiet(&commit_args)?;
    println!("Commit created");

    if push {
        print!("Pushing");
        let branch = get_current_branch().ok();
        if let Some(b) = branch {
            print!(" to {}", b);
        }
        println!("...");
        run_git_quiet(&["push"])?;
        println!("Pushed successfully");
    }

    println!("Done.");
    Ok(())
}
