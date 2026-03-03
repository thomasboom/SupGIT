use std::process::Command as StdCommand;

use anyhow::{Context, Result, bail};
use dialoguer::{Confirm, Input, Select};

use crate::git::run_git_silent;
use crate::status::{get_branches, get_current_branch};

pub fn create_branch(branch_name: &str) -> Result<()> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        bail!("branch name cannot be empty");
    }
    if branch_name.contains(|c: char| c.is_whitespace()) {
        bail!("branch name cannot contain whitespace");
    }
    run_git_silent(&["checkout", "-b", branch_name])?;
    println!("✓ Created and switched to branch '{}'", branch_name);
    Ok(())
}

pub fn delete_branch(branch_name: &str) -> Result<()> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        bail!("branch name cannot be empty");
    }

    let current = get_current_branch().unwrap_or_default();
    if branch_name == current {
        bail!(
            "cannot delete the current branch '{}'; switch to another branch first",
            branch_name
        );
    }

    let branches = get_branches()?;
    if !branches.iter().any(|b| b == branch_name) {
        bail!("branch '{}' does not exist", branch_name);
    }

    run_git_silent(&["branch", "-d", branch_name])?;
    println!("✓ Deleted branch '{}'", branch_name);
    Ok(())
}

pub fn delete_branch_interactive() -> Result<()> {
    let branches = get_branches()?;
    let current = get_current_branch().unwrap_or_default();

    let deletable_branches: Vec<String> = branches
        .iter()
        .filter(|b| b != &&current)
        .cloned()
        .collect();

    if deletable_branches.is_empty() {
        bail!("no branches available to delete (cannot delete current branch)");
    }

    let display_branches: Vec<String> = deletable_branches.to_vec();

    let selection = Select::new()
        .with_prompt("Select a branch to delete")
        .items(&display_branches)
        .default(0)
        .interact()?;

    let branch_to_delete = &deletable_branches[selection];

    let confirmed = Confirm::new()
        .with_prompt(format!("Delete branch '{}'?", branch_to_delete))
        .default(false)
        .interact()?;

    if confirmed {
        let output = StdCommand::new("git")
            .args(["branch", "-d", branch_to_delete])
            .output()
            .context("running git branch -d")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() && stderr.contains("not fully merged") {
            let force = Confirm::new()
                .with_prompt(format!(
                    "Branch '{}' is not fully merged. Force delete?",
                    branch_to_delete
                ))
                .default(false)
                .interact()?;

            if force {
                run_git_silent(&["branch", "-D", branch_to_delete])?;
                println!("✓ Force deleted branch '{}'", branch_to_delete);
            } else {
                println!("Cancelled.");
            }
        } else if !output.status.success() {
            bail!(
                "failed to delete branch '{}': {}",
                branch_to_delete,
                stderr.trim()
            );
        } else {
            println!("✓ Deleted branch '{}'", branch_to_delete);
        }
    } else {
        println!("Cancelled.");
    }

    Ok(())
}

pub fn run_branch_interactive() -> Result<()> {
    let branches = get_branches()?;
    let current = get_current_branch().unwrap_or_default();

    let mut display_branches: Vec<String> = branches
        .iter()
        .map(|b| {
            if b == &current {
                format!("{} (current)", b)
            } else {
                b.clone()
            }
        })
        .collect();
    display_branches.push("Create new branch...".to_string());
    display_branches.push("Delete a branch...".to_string());

    let selection = Select::new()
        .with_prompt("Select a branch to checkout")
        .items(&display_branches)
        .default(0)
        .interact()?;

    if selection == branches.len() {
        let branch_name: String = Input::new().with_prompt("New branch name").interact()?;

        if branch_name.is_empty() {
            bail!("branch name cannot be empty");
        }

        let normalized_name = branch_name.trim().replace(' ', "-");
        run_git_silent(&["checkout", "-b", &normalized_name])?;
        println!("✓ Created and switched to branch '{}'", normalized_name);
    } else if selection == branches.len() + 1 {
        delete_branch_interactive()?;
    } else {
        let selected_branch = &branches[selection];
        if selected_branch == &current {
            println!("Already on branch '{}'.", selected_branch);
        } else {
            run_git_silent(&["checkout", selected_branch])?;
            println!("✓ Switched to branch '{}'", selected_branch);
        }
    }

    Ok(())
}
