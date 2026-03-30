use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use dialoguer::Select;

const ALIAS_MARKER_START: &str = "# >>> supgit alias >>>";
const ALIAS_MARKER_END: &str = "# <<< supgit alias <<<";
const SG_ALIAS_MARKER_START: &str = "# >>> supgit sg alias >>>";
const SG_ALIAS_MARKER_END: &str = "# <<< supgit sg alias <<<";

pub fn run_alias(dry_run: bool, git: bool, sg: bool, non_interactive: bool) -> Result<()> {
    let shell_config = get_shell_config()?;

    if git && sg {
        anyhow::bail!("Cannot use both --git and --sg flags");
    }

    let alias_name = if git {
        "git".to_string()
    } else if sg {
        "sg".to_string()
    } else {
        if non_interactive {
            anyhow::bail!("alias selection requires --git or --sg flag in non-interactive mode");
        }

        let selection = Select::new()
            .with_prompt("Which alias would you like to add?")
            .item("git -> supgit")
            .item("sg -> supgit")
            .default(0)
            .interact()
            .context("failed to prompt for alias selection")?;

        if selection == 0 {
            "git".to_string()
        } else {
            "sg".to_string()
        }
    };

    if dry_run {
        println!("Would add alias to: {}", shell_config.display());
        println!("Alias: {} -> supgit", alias_name);
        return Ok(());
    }

    let existing_content = fs::read_to_string(&shell_config).unwrap_or_default();

    if alias_name == "git" {
        if existing_content.contains(ALIAS_MARKER_START) {
            println!("git alias already exists in {}", shell_config.display());
            return Ok(());
        }

        let alias_block = format!(
            "\n{}\nalias git='supgit'\n{}\n",
            ALIAS_MARKER_START, ALIAS_MARKER_END
        );

        let mut file = OpenOptions::new()
            .append(true)
            .open(&shell_config)
            .with_context(|| format!("failed to open {}", shell_config.display()))?;

        file.write_all(alias_block.as_bytes())
            .with_context(|| format!("failed to write to {}", shell_config.display()))?;

        println!("✓ Added 'git' alias to {}", shell_config.display());
    } else {
        if existing_content.contains("alias sg=") {
            println!("sg alias already exists in {}", shell_config.display());
            return Ok(());
        }

        let alias_block = format!(
            "\n{}\nalias sg='supgit'\n{}\n",
            SG_ALIAS_MARKER_START, SG_ALIAS_MARKER_END
        );

        let mut file = OpenOptions::new()
            .append(true)
            .open(&shell_config)
            .with_context(|| format!("failed to open {}", shell_config.display()))?;

        file.write_all(alias_block.as_bytes())
            .with_context(|| format!("failed to write to {}", shell_config.display()))?;

        println!("✓ Added 'sg' alias to {}", shell_config.display());
    }

    println!(
        "  Run 'source {}' or start a new shell for changes to take effect.",
        shell_config.display()
    );

    Ok(())
}

pub fn run_unalias(dry_run: bool, git: bool, sg: bool, non_interactive: bool) -> Result<()> {
    let shell_config = get_shell_config()?;

    if git && sg {
        anyhow::bail!("Cannot use both --git and --sg flags");
    }

    let alias_name = if git {
        "git".to_string()
    } else if sg {
        "sg".to_string()
    } else {
        if non_interactive {
            anyhow::bail!("alias selection requires --git or --sg flag in non-interactive mode");
        }

        let selection = Select::new()
            .with_prompt("Which alias would you like to remove?")
            .item("git")
            .item("sg")
            .default(0)
            .interact()
            .context("failed to prompt for alias selection")?;

        if selection == 0 {
            "git".to_string()
        } else {
            "sg".to_string()
        }
    };

    if dry_run {
        println!(
            "Would remove {} alias from: {}",
            alias_name,
            shell_config.display()
        );
        return Ok(());
    }

    let existing_content = fs::read_to_string(&shell_config)
        .with_context(|| format!("failed to read {}", shell_config.display()))?;

    if alias_name == "git" {
        if !existing_content.contains(ALIAS_MARKER_START) {
            println!("No git alias found in {}", shell_config.display());
            return Ok(());
        }

        let start_idx = existing_content
            .find(ALIAS_MARKER_START)
            .context("failed to find alias start marker")?;
        let end_idx = existing_content
            .find(ALIAS_MARKER_END)
            .context("failed to find alias end marker")?;

        let end_of_block = end_idx + ALIAS_MARKER_END.len();

        let new_content = if start_idx > 0 && existing_content[..start_idx].ends_with('\n') {
            let trimmed_start = start_idx - 1;
            format!(
                "{}{}",
                &existing_content[..trimmed_start],
                &existing_content[end_of_block..]
            )
        } else {
            format!(
                "{}{}",
                &existing_content[..start_idx],
                &existing_content[end_of_block..]
            )
        };

        fs::write(&shell_config, new_content.trim_end())
            .with_context(|| format!("failed to write to {}", shell_config.display()))?;

        println!("✓ Removed 'git' alias from {}", shell_config.display());
    } else {
        if !existing_content.contains(SG_ALIAS_MARKER_START) {
            println!("No sg alias found in {}", shell_config.display());
            return Ok(());
        }

        let start_idx = existing_content
            .find(SG_ALIAS_MARKER_START)
            .context("failed to find sg alias start marker")?;
        let end_idx = existing_content
            .find(SG_ALIAS_MARKER_END)
            .context("failed to find sg alias end marker")?;

        let end_of_block = end_idx + SG_ALIAS_MARKER_END.len();

        let new_content = if start_idx > 0 && existing_content[..start_idx].ends_with('\n') {
            let trimmed_start = start_idx - 1;
            format!(
                "{}{}",
                &existing_content[..trimmed_start],
                &existing_content[end_of_block..]
            )
        } else {
            format!(
                "{}{}",
                &existing_content[..start_idx],
                &existing_content[end_of_block..]
            )
        };

        fs::write(&shell_config, new_content.trim_end())
            .with_context(|| format!("failed to write to {}", shell_config.display()))?;

        println!("✓ Removed 'sg' alias from {}", shell_config.display());
    }

    println!(
        "  Run 'source {}' or start a new shell for changes to take effect.",
        shell_config.display()
    );

    Ok(())
}

fn get_shell_config() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .context("could not determine home directory")?;

    let home_path = PathBuf::from(&home);

    let shell = env::var("SHELL").unwrap_or_default();

    let config_name = if shell.contains("zsh") {
        ".zshrc"
    } else {
        ".bashrc"
    };

    Ok(home_path.join(config_name))
}
