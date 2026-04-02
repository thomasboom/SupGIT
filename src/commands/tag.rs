use std::process::Command as StdCommand;

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, Select};

use crate::git::run_git_silent;

pub struct TagInfo {
    pub name: String,
    pub message: Option<String>,
    pub is_annotated: bool,
}

pub fn get_tags() -> Result<Vec<TagInfo>> {
    let output = StdCommand::new("git")
        .args([
            "tag",
            "-n",
            "--format=%(refname:short)%(contents:subject) | %(objectname:short)",
        ])
        .output()
        .context("running git tag list")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tags: Vec<TagInfo> = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(" | ").collect();
        if parts.is_empty() {
            continue;
        }

        let name = parts[0].to_string();

        let (message, is_annotated) = if parts.len() > 1 && !parts[0].contains(" ") {
            (None, false)
        } else if name.contains(' ') {
            let msg_parts: Vec<&str> = name.splitn(2, ' ').collect();
            (Some(msg_parts.get(1).unwrap_or(&"").to_string()), true)
        } else {
            (None, false)
        };

        tags.push(TagInfo {
            name,
            message,
            is_annotated,
        });
    }

    Ok(tags)
}

pub fn create_tag(name: &str, message: Option<&str>, force: bool) -> Result<()> {
    let mut args = vec!["tag"];
    if force {
        args.push("-f");
    }

    if let Some(msg) = message {
        args.push("-a");
        args.push(name);
        args.push("-m");
        args.push(msg);
    } else {
        args.push(name);
    }

    run_git_silent(&args)?;
    if message.is_some() {
        println!("✓ Created annotated tag '{}'", name);
    } else {
        println!("✓ Created lightweight tag '{}'", name);
    }
    Ok(())
}

pub fn delete_tag(name: &str) -> Result<()> {
    run_git_silent(&["tag", "-d", name])?;
    println!("✓ Deleted tag '{}'", name);
    Ok(())
}

pub fn push_tag(name: &str, remote: Option<&str>) -> Result<()> {
    let mut args = vec!["push"];
    if let Some(r) = remote {
        args.push(r);
    }
    args.push("tag");
    args.push(name);

    run_git_silent(&args)?;
    println!("✓ Pushed tag '{}'", name);
    Ok(())
}

pub fn push_all_tags(remote: Option<&str>) -> Result<()> {
    let mut args = vec!["push"];
    if let Some(r) = remote {
        args.push(r);
    }
    args.push("--tags");

    run_git_silent(&args)?;
    println!("✓ Pushed all tags");
    Ok(())
}

pub fn checkout_tag(name: &str) -> Result<()> {
    run_git_silent(&["checkout", name])?;
    println!("✓ Switched to tag '{}'", name);
    Ok(())
}

pub fn run_tag_interactive(non_interactive: bool) -> Result<()> {
    let tags = get_tags()?;

    if non_interactive {
        if tags.is_empty() {
            println!("No tags found.");
            println!("Use 'supgit tag create <name>' to create one.");
        } else {
            println!("Tags:");
            for tag in &tags {
                let annotation = if tag.is_annotated { " (annotated)" } else { "" };
                if let Some(msg) = &tag.message {
                    println!("  {}{}: {}", tag.name, annotation, msg);
                } else {
                    println!("  {}{}", tag.name, annotation);
                }
            }
        }
        return Ok(());
    }

    let mut options: Vec<String> = Vec::new();
    if !tags.is_empty() {
        for tag in &tags {
            let annotation = if tag.is_annotated { " (annotated)" } else { "" };
            options.push(format!("{}{}", tag.name, annotation));
        }
        options.push("Delete a tag".to_string());
        options.push("Push a tag".to_string());
        options.push("Push all tags".to_string());
    }
    options.push("Create a new tag".to_string());

    let selection = Select::new()
        .with_prompt("Select a tag action")
        .items(&options)
        .default(0)
        .interact()?;

    let has_tags = !tags.is_empty();
    let delete_idx = if has_tags { tags.len() } else { 0 };
    let push_idx = delete_idx + 1;
    let push_all_idx = push_idx + 1;
    let create_idx = push_all_idx + 1;

    if has_tags && selection < tags.len() {
        let tag = &tags[selection];
        let confirmed = Confirm::new()
            .with_prompt(format!("Checkout tag '{}'?", tag.name))
            .default(false)
            .interact()?;
        if confirmed {
            checkout_tag(&tag.name)?;
        }
    } else if (!has_tags && selection == 0) || (has_tags && selection == delete_idx) {
        if tags.is_empty() {
            println!("No tags to delete.");
            return Ok(());
        }
        let selection = Select::new()
            .with_prompt("Select a tag to delete")
            .items(&tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>())
            .default(0)
            .interact()?;
        delete_tag(&tags[selection].name)?;
    } else if (!has_tags && selection == 1) || (has_tags && selection == push_idx) {
        if tags.is_empty() {
            println!("No tags to push.");
            return Ok(());
        }
        let selection = Select::new()
            .with_prompt("Select a tag to push")
            .items(&tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>())
            .default(0)
            .interact()?;
        push_tag(&tags[selection].name, None)?;
    } else if (!has_tags && selection == 2) || (has_tags && selection == push_all_idx) {
        push_all_tags(None)?;
    } else if (!has_tags && selection == 3) || (has_tags && selection == create_idx) {
        let name: String = Input::new().with_prompt("Tag name").interact()?;

        if name.trim().is_empty() {
            println!("Tag name cannot be empty.");
            return Ok(());
        }

        let is_annotated = Confirm::new()
            .with_prompt("Create annotated tag?")
            .default(true)
            .interact()?;

        if is_annotated {
            let message: String = Input::new().with_prompt("Tag message").interact()?;
            create_tag(&name, Some(&message), false)?;
        } else {
            create_tag(&name, None, false)?;
        }
    }

    Ok(())
}
