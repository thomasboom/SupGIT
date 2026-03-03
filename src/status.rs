use std::cell::RefCell;
use std::process::Command as StdCommand;

use anyhow::{bail, Context, Result};

use crate::git::NOT_IN_REPO_HINT;

thread_local! {
    static PORCELAIN_CACHE: RefCell<Option<Vec<(String, String)>>> = const { RefCell::new(None) };
}

fn get_porcelain_lines_cached() -> Result<Vec<(String, String)>> {
    PORCELAIN_CACHE.with(|cache| {
        if let Some(ref entries) = *cache.borrow() {
            return Ok(entries.clone());
        }

        let output = StdCommand::new("git")
            .args(["status", "--porcelain"])
            .output()
            .context("running git status --porcelain")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git status --porcelain failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<(String, String)> = stdout
            .lines()
            .filter_map(|line| {
                if line.len() < 4 {
                    return None;
                }
                let status = line[..2].to_string();
                let path = line[3..].to_string();
                Some((status, path))
            })
            .collect();

        *cache.borrow_mut() = Some(entries.clone());
        Ok(entries)
    })
}

pub fn invalidate_porcelain_cache() {
    PORCELAIN_CACHE.with(|cache| {
        *cache.borrow_mut() = None;
    });
}

pub fn get_repo_root() -> Result<String> {
    let output = StdCommand::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to execute git - is git installed?")?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout);
        let path = path.trim().to_string();
        if path.is_empty() {
            bail!("{}", NOT_IN_REPO_HINT);
        }
        Ok(path)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            bail!("{}", NOT_IN_REPO_HINT);
        }
        bail!("failed to get repo root: {}", stderr.trim());
    }
}

pub struct PorcelainStatus {
    entries: Vec<(String, String)>,
}

impl PorcelainStatus {
    pub fn parse() -> Result<Self> {
        Ok(Self {
            entries: get_porcelain_lines_cached()?,
        })
    }

    pub fn unstaged_files(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(status, _)| {
                let xy: Vec<char> = status.chars().collect();
                let x = xy.first().copied().unwrap_or(' ');
                let y = xy.get(1).copied().unwrap_or(' ');
                x == ' ' && y != ' ' && y != '?'
            })
            .map(|(_, path)| path.as_str())
            .collect()
    }

    pub fn all_uncommitted_files(&self) -> Vec<&str> {
        self.entries.iter().map(|(_, path)| path.as_str()).collect()
    }
}

pub fn get_porcelain_lines() -> Result<Vec<(String, String)>> {
    get_porcelain_lines_cached()
}

pub fn get_unstaged_files() -> Result<Vec<String>> {
    let entries = get_porcelain_lines()?;
    let files: Vec<String> = entries
        .iter()
        .filter(|(status, _)| {
            let xy: Vec<char> = status.chars().collect();
            let x = xy.first().copied().unwrap_or(' ');
            let y = xy.get(1).copied().unwrap_or(' ');
            x == ' ' && y != ' ' && y != '?'
        })
        .map(|(_, path)| path.clone())
        .collect();

    Ok(files)
}

pub fn get_staged_files() -> Result<Vec<String>> {
    let entries = get_porcelain_lines()?;
    let files: Vec<String> = entries
        .iter()
        .filter(|(status, _)| {
            let x = status.chars().next().unwrap_or(' ');
            matches!(x, 'M' | 'A' | 'D' | 'R' | 'C')
        })
        .map(|(_, path)| path.clone())
        .collect();

    Ok(files)
}

pub fn get_all_uncommitted_files() -> Result<Vec<String>> {
    let entries = get_porcelain_lines()?;
    let files: Vec<String> = entries.iter().map(|(_, path)| path.clone()).collect();
    Ok(files)
}

pub fn get_untracked_files() -> Result<Vec<String>> {
    let entries = get_porcelain_lines()?;
    let files: Vec<String> = entries
        .iter()
        .filter(|(status, _)| {
            let xy: Vec<char> = status.chars().collect();
            let x = xy.first().copied().unwrap_or(' ');
            let y = xy.get(1).copied().unwrap_or(' ');
            x == '?' && y == '?'
        })
        .map(|(_, path)| path.clone())
        .collect();
    Ok(files)
}

pub fn get_branches() -> Result<Vec<String>> {
    let output = StdCommand::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .context("running git branch")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(branches)
}

pub fn get_current_branch() -> Result<String> {
    let output = StdCommand::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("getting current branch")?;

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch)
}
