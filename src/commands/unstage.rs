use anyhow::Result;
use dialoguer::{MultiSelect, Select};

use crate::git::run_git_silent;
use crate::status::{get_repo_root, get_staged_files};

pub fn restore_stage(targets: &[String], all: bool, non_interactive: bool) -> Result<()> {
    let is_interactive = targets.is_empty() && !all;

    if is_interactive {
        if non_interactive {
            run_git_silent(&["restore", "--staged", "."])?;
            println!("✓ All files unstaged (non-interactive mode)");
            return Ok(());
        }

        let selection = Select::new()
            .with_prompt("What would you like to unstage?")
            .items(&["All staged files", "Specific files"])
            .default(0)
            .interact()?;

        match selection {
            0 => {
                run_git_silent(&["restore", "--staged", "."])?;
                println!("✓ All files unstaged");
                Ok(())
            }
            1 => {
                let files = get_staged_files()?;
                if files.is_empty() {
                    println!("No staged files to unstage.");
                    return Ok(());
                }
                let selected = MultiSelect::new()
                    .with_prompt("Select files to unstage")
                    .items(&files)
                    .interact()?;

                if selected.is_empty() {
                    println!("No files selected.");
                    return Ok(());
                }

                let repo_root = get_repo_root()?;
                let mut args = vec!["restore".to_string(), "--staged".to_string()];
                let count = selected.len();
                for idx in selected {
                    args.push(files[idx].clone());
                }
                let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                crate::git::run_git_in_dir_silent(&args_refs, &repo_root)?;
                println!("✓ Unstaged {} file(s)", count);
                Ok(())
            }
            _ => Ok(()),
        }
    } else if all {
        run_git_silent(&["restore", "--staged", "."])?;
        println!("✓ All files unstaged");
        Ok(())
    } else {
        let target_args: Vec<&str> = if targets.is_empty() {
            vec!["."]
        } else {
            targets.iter().map(String::as_str).collect()
        };

        let mut args = Vec::with_capacity(2 + target_args.len());
        args.push("restore");
        args.push("--staged");
        args.extend(target_args);

        run_git_silent(&args)?;
        println!("✓ Files unstaged");
        Ok(())
    }
}
