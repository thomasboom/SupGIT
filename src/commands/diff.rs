use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

use anyhow::{Context, Result, bail};
use dialoguer::Select;

use crate::status::get_porcelain_lines;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChangeType {
    Modified,
    Created,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
}

impl ChangeType {
    fn label(self) -> &'static str {
        match self {
            ChangeType::Modified => "modified",
            ChangeType::Created => "created",
            ChangeType::Deleted => "deleted",
            ChangeType::Renamed => "renamed",
            ChangeType::Copied => "copied",
            ChangeType::TypeChanged => "type-changed",
            ChangeType::Unmerged => "unmerged",
            ChangeType::Untracked => "created",
        }
    }
}

struct FileDiffEntry {
    display_path: String,
    git_path: String,
    change_type: ChangeType,
    additions: Option<usize>,
    deletions: Option<usize>,
}

pub fn run_diff(path: Option<String>, staged: bool) -> Result<()> {
    if let Some(path) = path {
        show_diff_for_path(&path, staged, false)?;
        return Ok(());
    }

    run_diff_selector(staged)
}

fn run_diff_selector(staged: bool) -> Result<()> {
    let entries = build_diff_entries(staged)?;
    if entries.is_empty() {
        if staged {
            println!("No staged files to diff.");
        } else {
            println!("No unstaged files to diff.");
        }
        return Ok(());
    }

    let mut items: Vec<String> = entries.iter().map(format_selector_item).collect();
    items.push("Cancel".to_string());

    let prompt = if staged {
        "Select a staged file to view its diff"
    } else {
        "Select an unstaged file to view its diff"
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()?;

    if selection == entries.len() {
        println!("No file selected.");
        return Ok(());
    }

    let selected = &entries[selection];
    show_diff_for_path(
        &selected.git_path,
        staged,
        selected.change_type == ChangeType::Untracked,
    )?;
    Ok(())
}

fn build_diff_entries(staged: bool) -> Result<Vec<FileDiffEntry>> {
    let porcelain_entries = get_porcelain_lines()?;
    let mut entries = Vec::new();

    for (status, path) in porcelain_entries {
        if let Some(change_type) = classify_change(&status, staged) {
            let git_path = canonical_git_path(&path);
            let (additions, deletions) =
                get_line_change_counts(&git_path, staged, change_type == ChangeType::Untracked)?;
            entries.push(FileDiffEntry {
                display_path: path,
                git_path,
                change_type,
                additions,
                deletions,
            });
        }
    }

    Ok(entries)
}

fn classify_change(status: &str, staged: bool) -> Option<ChangeType> {
    let chars: Vec<char> = status.chars().collect();
    let x = chars.first().copied().unwrap_or(' ');
    let y = chars.get(1).copied().unwrap_or(' ');

    if !staged && x == '?' && y == '?' {
        return Some(ChangeType::Untracked);
    }

    let code = if staged { x } else { y };
    match code {
        'M' => Some(ChangeType::Modified),
        'A' => Some(ChangeType::Created),
        'D' => Some(ChangeType::Deleted),
        'R' => Some(ChangeType::Renamed),
        'C' => Some(ChangeType::Copied),
        'T' => Some(ChangeType::TypeChanged),
        'U' => Some(ChangeType::Unmerged),
        _ => None,
    }
}

fn canonical_git_path(path: &str) -> String {
    if let Some((_, new_path)) = path.split_once(" -> ") {
        return new_path.to_string();
    }
    path.to_string()
}

fn get_line_change_counts(
    path: &str,
    staged: bool,
    untracked: bool,
) -> Result<(Option<usize>, Option<usize>)> {
    if untracked {
        return Ok((count_file_lines(path).ok(), Some(0)));
    }

    let mut args = vec!["diff", "--numstat"];
    if staged {
        args.push("--staged");
    }
    args.push("--");
    args.push(path);

    let output = StdCommand::new("git")
        .args(&args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed:\n  {}", args.join(" "), stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().next() {
        let mut parts = line.split('\t');
        let additions = parts.next().and_then(parse_numstat_value);
        let deletions = parts.next().and_then(parse_numstat_value);
        return Ok((additions, deletions));
    }

    Ok((Some(0), Some(0)))
}

fn parse_numstat_value(raw: &str) -> Option<usize> {
    if raw == "-" {
        None
    } else {
        raw.parse::<usize>().ok()
    }
}

fn count_file_lines(path: &str) -> Result<usize> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path))?;
    if bytes.contains(&0) {
        bail!("binary file");
    }
    let text = String::from_utf8_lossy(&bytes);
    Ok(text.lines().count())
}

fn format_selector_item(entry: &FileDiffEntry) -> String {
    let stat_text = match (entry.additions, entry.deletions) {
        (Some(add), Some(del)) => format!("+{} -{}", add, del),
        _ => "binary".to_string(),
    };
    format!(
        "{} [{}] ({})",
        entry.display_path,
        entry.change_type.label(),
        stat_text
    )
}

fn show_diff_for_path(path: &str, staged: bool, untracked: bool) -> Result<()> {
    if untracked {
        show_untracked_diff(path)?;
    } else {
        show_tracked_diff(path, staged)?;
    }
    Ok(())
}

fn show_tracked_diff(path: &str, staged: bool) -> Result<()> {
    let mut args = vec!["-c", "color.ui=always", "diff"];
    if staged {
        args.push("--staged");
    }
    args.push("--");
    args.push(path);

    let output = StdCommand::new("git")
        .args(&args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed:\n  {}", args.join(" "), stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        println!("No diff output for '{}'.", path);
    } else {
        print!("{}", stdout);
    }

    Ok(())
}

fn show_untracked_diff(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        bail!("file '{}' no longer exists", path);
    }

    let output = StdCommand::new("git")
        .args([
            "diff",
            "--no-index",
            "--color=always",
            "--",
            "/dev/null",
            path,
        ])
        .output()
        .context("running git diff --no-index for untracked file")?;

    if !output.status.success() && output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff --no-index failed:\n  {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        println!("No diff output for '{}'.", path);
    } else {
        print!("{}", stdout);
    }
    Ok(())
}
