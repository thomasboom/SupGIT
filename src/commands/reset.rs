use anyhow::Result;
use dialoguer::{FuzzySelect, MultiSelect};

use crate::git::run_git_silent;
use crate::status::{
    get_all_uncommitted_files, get_porcelain_lines, get_repo_root, get_staged_files,
    get_unstaged_files, get_untracked_files,
};

pub fn run_reset(
    all: bool,
    staged: bool,
    unstaged: bool,
    tracked: bool,
    untracked: bool,
    non_interactive: bool,
) -> Result<()> {
    let is_interactive = !all && !staged && !unstaged && !tracked && !untracked;

    if is_interactive {
        if non_interactive {
            reset_all()?;
            println!("(non-interactive mode)");
            return Ok(());
        }

        let selection = FuzzySelect::new()
            .with_prompt("What would you like to reset?")
            .items(&[
                "All files",
                "Staged files only",
                "Unstaged changes only",
                "Tracked files only",
                "Untracked files only",
                "Custom files",
            ])
            .default(0)
            .interact()?;

        match selection {
            0 => reset_all()?,
            1 => reset_staged()?,
            2 => reset_unstaged()?,
            3 => reset_tracked()?,
            4 => reset_untracked()?,
            5 => reset_custom()?,
            _ => {}
        }
    } else if all {
        reset_all()?;
    } else if staged {
        reset_staged()?;
    } else if unstaged {
        reset_unstaged()?;
    } else if tracked {
        reset_tracked()?;
    } else if untracked {
        reset_untracked()?;
    }

    Ok(())
}

fn reset_all() -> Result<()> {
    run_git_silent(&["reset", "--hard"])?;
    run_git_silent(&["clean", "-fd"])?;
    println!("✓ All files reset.");
    Ok(())
}

fn reset_staged() -> Result<()> {
    let files = get_staged_files()?;
    if files.is_empty() {
        println!("No staged files to reset.");
        return Ok(());
    }
    run_git_silent(&["restore", "--staged", "."])?;
    println!("✓ Staged files reset.");
    Ok(())
}

fn reset_unstaged() -> Result<()> {
    let files = get_unstaged_files()?;
    if files.is_empty() {
        println!("No unstaged changes to reset.");
        return Ok(());
    }
    run_git_silent(&["restore", "."])?;
    println!("✓ Unstaged changes reset.");
    Ok(())
}

fn reset_tracked() -> Result<()> {
    run_git_silent(&["reset", "--hard"])?;
    println!("✓ Tracked files reset.");
    Ok(())
}

fn reset_untracked() -> Result<()> {
    let files = get_untracked_files()?;
    if files.is_empty() {
        println!("No untracked files to reset.");
        return Ok(());
    }
    run_git_silent(&["clean", "-fd"])?;
    println!("✓ Untracked files removed.");
    Ok(())
}

fn reset_custom() -> Result<()> {
    let files = get_all_uncommitted_files()?;
    if files.is_empty() {
        println!("No files to reset.");
        return Ok(());
    }

    let selected = MultiSelect::new()
        .with_prompt("Select files to reset")
        .items(&files)
        .interact()?;

    if selected.is_empty() {
        println!("No files selected.");
        return Ok(());
    }

    let repo_root = get_repo_root()?;
    for idx in selected {
        let file = &files[idx];
        let entries = get_porcelain_lines()?;
        let status = entries
            .iter()
            .find(|(_, p)| p == file)
            .map(|(s, _)| s.clone())
            .unwrap_or_default();
        let xy: Vec<char> = status.chars().collect();
        let x = xy.first().copied().unwrap_or(' ');
        let y = xy.get(1).copied().unwrap_or(' ');

        if x == '?' && y == '?' {
            crate::git::run_git_in_dir_silent(&["clean", "-f", file], &repo_root)?;
        } else {
            if x != ' ' {
                crate::git::run_git_in_dir_silent(&["restore", "--staged", file], &repo_root)?;
            }
            if y != ' ' && y != '?' {
                crate::git::run_git_in_dir_silent(&["restore", file], &repo_root)?;
            }
        }
    }

    println!("✓ Selected files reset.");
    Ok(())
}
