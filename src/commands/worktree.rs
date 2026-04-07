use std::process::Command as StdCommand;

use anyhow::{bail, Context, Result};
use dialoguer::{Confirm, FuzzySelect, Input};

use crate::git::run_git_silent;

pub struct WorktreeInfo {
    pub path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
}

pub fn get_worktrees() -> Result<Vec<WorktreeInfo>> {
    let output = StdCommand::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("running git worktree list")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees: Vec<WorktreeInfo> = Vec::new();
    let mut current_worktree: Option<WorktreeInfo> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(wt) = current_worktree.take() {
                worktrees.push(wt);
            }
            continue;
        }

        if line.starts_with("worktree ") {
            let path = line
                .strip_prefix("worktree ")
                .unwrap_or("")
                .trim()
                .to_string();
            current_worktree = Some(WorktreeInfo {
                path,
                branch: None,
                head: None,
            });
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current_worktree {
                wt.branch = Some(
                    line.strip_prefix("branch ")
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                );
            }
        } else if line.starts_with("HEAD ") {
            if let Some(ref mut wt) = current_worktree {
                wt.head = Some(line.strip_prefix("HEAD ").unwrap_or("").trim().to_string());
            }
        }
    }

    if let Some(wt) = current_worktree {
        worktrees.push(wt);
    }

    Ok(worktrees)
}

pub fn create_worktree(path: &str, branch: Option<&str>, create_branch: bool) -> Result<()> {
    let mut args = vec!["worktree", "add"];

    if create_branch {
        args.push("-b");
    }

    args.push(path);

    if let Some(branch_name) = branch {
        args.push(branch_name);
    }

    run_git_silent(&args)?;
    println!("✓ Created worktree at '{}'", path);
    Ok(())
}

pub fn remove_worktree(path: &str, force: bool) -> Result<()> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path);

    run_git_silent(&args)?;
    println!("✓ Removed worktree at '{}'", path);
    Ok(())
}

pub fn prune_worktrees() -> Result<()> {
    run_git_silent(&["worktree", "prune"])?;
    println!("✓ Pruned worktrees");
    Ok(())
}

pub fn run_worktree_interactive(non_interactive: bool) -> Result<()> {
    let worktrees = get_worktrees()?;

    if non_interactive {
        if worktrees.is_empty() {
            println!("No worktrees found.");
            println!("Use 'supgit worktree add <path> [--branch <name>]' to create one.");
        } else {
            println!("Worktrees:");
            for wt in &worktrees {
                let info = wt.branch.as_ref().or(wt.head.as_ref());
                if let Some(info) = info {
                    println!(
                        "  {} ({}: {})",
                        wt.path,
                        if wt.branch.is_some() {
                            "branch"
                        } else {
                            "detached"
                        },
                        info
                    );
                } else {
                    println!("  {}", wt.path);
                }
            }
        }
        return Ok(());
    }

    let mut options: Vec<String> = Vec::new();
    if !worktrees.is_empty() {
        for wt in &worktrees {
            let info = wt.branch.as_ref().or(wt.head.as_ref());
            if let Some(info) = info {
                options.push(format!("{} ({})", wt.path, info));
            } else {
                options.push(wt.path.clone());
            }
        }
        options.push("Remove a worktree".to_string());
    }
    options.push("Create a new worktree".to_string());
    options.push("Prune stale worktrees".to_string());

    let selection = FuzzySelect::new()
        .with_prompt("Select a worktree action")
        .items(&options)
        .default(0)
        .interact()?;

    let has_worktrees = !worktrees.is_empty();
    let remove_idx = if has_worktrees { worktrees.len() } else { 0 };
    let create_idx = remove_idx + 1;
    let prune_idx = create_idx + 1;

    if has_worktrees && selection < worktrees.len() {
        let wt = &worktrees[selection];
        let confirmed = Confirm::new()
            .with_prompt(format!("Remove worktree at '{}'?", wt.path))
            .default(false)
            .interact()?;
        if confirmed {
            remove_worktree(&wt.path, false)?;
        } else {
            println!("Cancelled.");
        }
    } else if (!has_worktrees && selection == 0) || (has_worktrees && selection == remove_idx) {
        if worktrees.is_empty() {
            println!("No worktrees to remove.");
            return Ok(());
        }
        let selection = FuzzySelect::new()
            .with_prompt("Select a worktree to remove")
            .items(
                &worktrees
                    .iter()
                    .map(|wt| wt.path.clone())
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()?;
        remove_worktree(&worktrees[selection].path, false)?;
    } else if (!has_worktrees && selection == 1) || (has_worktrees && selection == create_idx) {
        let path: String = Input::new()
            .with_prompt("Worktree path (e.g., ../my-feature)")
            .interact()?;

        if path.trim().is_empty() {
            println!("Path cannot be empty.");
            return Ok(());
        }

        let create_new_branch = Confirm::new()
            .with_prompt("Create a new branch?")
            .default(false)
            .interact()?;

        let branch_name: Option<String> = if create_new_branch {
            let name: String = Input::new().with_prompt("Branch name").interact()?;
            if name.trim().is_empty() {
                println!("Branch name cannot be empty.");
                return Ok(());
            }
            Some(name)
        } else {
            let use_existing = Confirm::new()
                .with_prompt("Use existing branch?")
                .default(true)
                .interact()?;

            if use_existing {
                let branch_output = StdCommand::new("git")
                    .args(["branch", "-a"])
                    .output()
                    .context("running git branch")?;
                let branches: Vec<String> = String::from_utf8_lossy(&branch_output.stdout)
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if branches.is_empty() {
                    bail!("No branches available");
                }

                let branch_selection = FuzzySelect::new()
                    .with_prompt("Select a branch")
                    .items(&branches)
                    .default(0)
                    .interact()?;
                Some(
                    branches[branch_selection]
                        .trim_start_matches("* ")
                        .to_string(),
                )
            } else {
                None
            }
        };

        create_worktree(&path, branch_name.as_deref(), create_new_branch)?;
    } else if (!has_worktrees && selection == 2) || (has_worktrees && selection == prune_idx) {
        prune_worktrees()?;
    }

    Ok(())
}
