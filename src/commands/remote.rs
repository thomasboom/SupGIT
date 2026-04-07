use std::process::Command as StdCommand;

use anyhow::{bail, Context, Result};
use dialoguer::{Confirm, FuzzySelect, Input};

use crate::git::run_git_silent;

pub struct RemoteInfo {
    pub name: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

pub fn get_remotes() -> Result<Vec<RemoteInfo>> {
    let output = StdCommand::new("git")
        .args(["remote", "-v"])
        .output()
        .context("running git remote -v")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut remotes: Vec<RemoteInfo> = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }

        let name = parts[0].to_string();
        let url_part = parts[1];

        let (url, remote_type) = if url_part.ends_with("(push)") {
            let end = url_part.len() - 7;
            (&url_part[..end], "push")
        } else if url_part.ends_with("(fetch)") {
            let end = url_part.len() - 7;
            (&url_part[..end], "fetch")
        } else {
            continue;
        };

        let url = url.to_string();

        if let Some(existing) = remotes.iter_mut().find(|r| r.name == name) {
            if remote_type == "fetch" {
                existing.fetch_url = Some(url);
            } else {
                existing.push_url = Some(url);
            }
        } else {
            let mut new_remote = RemoteInfo {
                name,
                fetch_url: None,
                push_url: None,
            };
            if remote_type == "fetch" {
                new_remote.fetch_url = Some(url);
            } else {
                new_remote.push_url = Some(url);
            }
            remotes.push(new_remote);
        }
    }

    Ok(remotes)
}

pub fn add_remote(name: &str, url: &str) -> Result<()> {
    let name = name.trim();
    let url = url.trim();

    if name.is_empty() {
        bail!("remote name cannot be empty");
    }
    if name.contains(|c: char| c.is_whitespace()) {
        bail!("remote name cannot contain whitespace");
    }
    if url.is_empty() {
        bail!("remote URL cannot be empty");
    }

    run_git_silent(&["remote", "add", name, url])?;
    println!("✓ Added remote '{}' ({})", name, url);
    Ok(())
}

pub fn remove_remote(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("remote name cannot be empty");
    }

    run_git_silent(&["remote", "remove", name])?;
    println!("✓ Removed remote '{}'", name);
    Ok(())
}

pub fn set_remote_url(name: &str, url: &str) -> Result<()> {
    let name = name.trim();
    let url = url.trim();

    if name.is_empty() {
        bail!("remote name cannot be empty");
    }
    if url.is_empty() {
        bail!("new remote URL cannot be empty");
    }

    run_git_silent(&["remote", "set-url", name, url])?;
    println!("✓ Set URL of '{}' to '{}'", name, url);
    Ok(())
}

pub fn run_remote_interactive(non_interactive: bool) -> Result<()> {
    let remotes = get_remotes()?;

    if non_interactive {
        if remotes.is_empty() {
            println!("No remotes configured.");
            println!("Use 'supgit remote add <name> <url>' to add a remote.");
        } else {
            println!("Configured remotes:");
            for remote in &remotes {
                let fetch = remote.fetch_url.as_deref().unwrap_or("(not set)");
                let push = remote.push_url.as_deref().unwrap_or("(not set)");
                println!("  {}: fetch={}, push={}", remote.name, fetch, push);
            }
        }
        return Ok(());
    }

    let mut options: Vec<String> = Vec::new();
    if !remotes.is_empty() {
        for remote in &remotes {
            let fetch = remote.fetch_url.as_deref().unwrap_or("(not set)");
            options.push(format!("{} ({})", remote.name, fetch));
        }
    }
    options.push("Add a new remote...".to_string());
    options.push("Remove a remote...".to_string());
    options.push("Change remote URL...".to_string());

    let selection = FuzzySelect::new()
        .with_prompt("Select a remote")
        .items(&options)
        .default(0)
        .interact()?;

    if selection < remotes.len() {
        let remote = &remotes[selection];
        let fetch = remote.fetch_url.as_deref().unwrap_or("(not set)");
        let push = remote.push_url.as_deref().unwrap_or("(not set)");
        println!("\nRemote: {}", remote.name);
        println!("  Fetch: {}", fetch);
        println!("  Push:  {}", push);
    } else if selection == remotes.len() {
        let name: String = Input::new().with_prompt("Remote name").interact()?;

        if name.trim().is_empty() {
            bail!("remote name cannot be empty");
        }

        let url: String = Input::new().with_prompt("Repository URL").interact()?;

        if url.trim().is_empty() {
            bail!("remote URL cannot be empty");
        }

        add_remote(&name, &url)?;
    } else if selection == remotes.len() + 1 {
        if remotes.is_empty() {
            bail!("no remotes to remove");
        }

        let remote_names: Vec<String> = remotes.iter().map(|r| r.name.clone()).collect();
        let selection = FuzzySelect::new()
            .with_prompt("Select a remote to remove")
            .items(&remote_names)
            .default(0)
            .interact()?;

        let name = &remote_names[selection];
        let confirmed = Confirm::new()
            .with_prompt(format!("Remove remote '{}'?", name))
            .default(false)
            .interact()?;

        if confirmed {
            remove_remote(name)?;
        } else {
            println!("Cancelled.");
        }
    } else {
        if remotes.is_empty() {
            bail!("no remotes to modify");
        }

        let remote_names: Vec<String> = remotes.iter().map(|r| r.name.clone()).collect();
        let selection = FuzzySelect::new()
            .with_prompt("Select a remote to change URL")
            .items(&remote_names)
            .default(0)
            .interact()?;

        let name = &remote_names[selection];
        let new_url: String = Input::new()
            .with_prompt(format!("New URL for '{}'", name))
            .interact()?;

        if new_url.trim().is_empty() {
            bail!("new remote URL cannot be empty");
        }

        set_remote_url(name, &new_url)?;
    }

    Ok(())
}
