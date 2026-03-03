use anyhow::Result;
use dialoguer::{MultiSelect, Select};

use crate::git::run_git_silent;
use crate::status::{PorcelainStatus, get_repo_root};

pub fn stage_targets(targets: &[String], all: bool, tracked: bool) -> Result<()> {
    let is_interactive = targets.is_empty() && !all && !tracked;

    if is_interactive {
        let selection = Select::new()
            .with_prompt("What would you like to stage?")
            .items(&["All files", "Tracked files only", "Specific files"])
            .default(0)
            .interact()?;

        match selection {
            0 => {
                run_git_silent(&["add", "-A"])?;
                println!("✓ Staged all files");
                Ok(())
            }
            1 => {
                run_git_silent(&["add", "-u"])?;
                println!("✓ Staged tracked files");
                Ok(())
            }
            2 => {
                let status = PorcelainStatus::parse()?;
                let files: Vec<&str> = status.unstaged_files();
                if files.is_empty() {
                    println!("No unstaged files to stage.");
                    return Ok(());
                }
                let files_owned: Vec<String> = files.iter().map(|s| s.to_string()).collect();
                let selected = MultiSelect::new()
                    .with_prompt("Select files to stage")
                    .items(&files_owned)
                    .interact()?;

                if selected.is_empty() {
                    println!("No files selected.");
                    return Ok(());
                }

                let repo_root = get_repo_root()?;
                let mut args = vec!["add".to_string()];
                let count = selected.len();
                for idx in selected {
                    args.push(files_owned[idx].clone());
                }
                let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                crate::git::run_git_in_dir_silent(&args_refs, &repo_root)?;
                println!("✓ Staged {} file(s)", count);
                Ok(())
            }
            _ => Ok(()),
        }
    } else if all {
        run_git_silent(&["add", "-A"])?;
        println!("✓ Staged all files");
        Ok(())
    } else if tracked {
        run_git_silent(&["add", "-u"])?;
        println!("✓ Staged tracked files");
        Ok(())
    } else {
        let target_args: Vec<&str> = if targets.is_empty() {
            vec!["."]
        } else {
            targets.iter().map(String::as_str).collect()
        };

        let mut args = Vec::with_capacity(1 + target_args.len());
        args.push("add");
        args.extend(target_args);

        run_git_silent(&args)?;
        println!("✓ Staged files");
        Ok(())
    }
}
