mod cli;
mod commands;
mod git;
mod status;

use anyhow::{bail, Result};
use clap::Parser;
use cli::{Cli, SupgitCommand};
use commands::{
    check_and_auto_update, create_branch, delete_branch, restore_stage, run_alias,
    run_branch_interactive, run_clone, run_commit, run_pull, run_push, run_reset, run_self_update,
    run_sync, run_unalias, stage_targets,
};
use git::{check_in_repo, run_git, run_git_silent};
use strsim::jaro_winkler;

const COMMANDS: &[&str] = &[
    "init", "stage", "unstage", "status", "commit", "log", "diff", "reset", "branch", "push",
    "pull", "sync", "clone", "update", "alias", "unalias",
];

fn find_closest_command(input: &str) -> Option<&'static str> {
    COMMANDS
        .iter()
        .map(|&cmd| (cmd, jaro_winkler(input, cmd)))
        .filter(|(_, score)| *score > 0.7)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(cmd, _)| cmd)
}

fn extract_unrecognized_subcommand(err_str: &str) -> Option<&str> {
    if err_str.contains("unrecognized subcommand") {
        let start = err_str.find("'")?;
        let rest = &err_str[start + 1..];
        let end = rest.find("'")?;
        Some(&rest[..end])
    } else {
        None
    }
}

fn main() {
    if let Err(err) = run() {
        for cause in err.chain() {
            eprintln!("error: {}", cause);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    check_and_auto_update();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let err_string = err.to_string();
            if let Some(unrecognized) = extract_unrecognized_subcommand(&err_string)
                && let Some(closest) = find_closest_command(unrecognized)
            {
                eprintln!("error: unrecognized subcommand '{}'", unrecognized);
                eprintln!("tip: running '{}' instead...", closest);
                let args = std::env::args().collect::<Vec<_>>();
                let mut new_args = vec![args[0].clone(), closest.to_string()];
                if let Some(pos) = args.iter().position(|a| a == unrecognized) {
                    new_args.extend(args[pos + 1..].iter().cloned());
                }
                let cli = Cli::parse_from(&new_args);
                if let Some(command) = cli.command {
                    return execute_command(command);
                }
            }
            err.exit();
        }
    };

    if cli.explain {
        print_explanations();
        return Ok(());
    }

    let command = match cli.command {
        Some(command) => command,
        None => bail!("'supgit' requires a subcommand; use --help to see the available list"),
    };

    execute_command(command)
}

fn execute_command(command: SupgitCommand) -> Result<()> {
    if !matches!(
        command,
        SupgitCommand::Init
            | SupgitCommand::Clone { .. }
            | SupgitCommand::Update
            | SupgitCommand::Alias { .. }
            | SupgitCommand::Unalias { .. }
    ) {
        check_in_repo()?;
    }

    match command {
        SupgitCommand::Init => {
            run_git_silent(&["init"])?;
            println!("✓ Initialized Git repository");
        }
        SupgitCommand::Stage {
            targets,
            all,
            tracked,
        } => stage_targets(&targets, all, tracked)?,
        SupgitCommand::Unstage { targets, all } => restore_stage(&targets, all)?,
        SupgitCommand::Status { short } => {
            if short {
                run_git(&["status", "-sb"])?;
            } else {
                run_git(&["status"])?;
            }
        }
        SupgitCommand::Log { short } => {
            if short {
                run_git(&["log", "--oneline", "--decorate", "-n", "20"])?;
            } else {
                run_git(&["log", "--decorate", "-n", "40"])?;
            }
        }
        SupgitCommand::Diff { path, staged } => {
            if staged {
                run_git(&["diff", "--staged"])?;
            } else if let Some(path) = path {
                run_git(&["diff", path.as_str()])?;
            } else {
                run_git(&["diff"])?;
            }
        }
        SupgitCommand::Reset {
            all,
            staged,
            unstaged,
            tracked,
            untracked,
        } => run_reset(all, staged, unstaged, tracked, untracked)?,
        SupgitCommand::Branch { create, delete } => {
            if let Some(branch_name) = create {
                create_branch(&branch_name)?;
            } else if let Some(branch_name) = delete {
                delete_branch(&branch_name)?;
            } else {
                run_branch_interactive()?;
            }
        }
        SupgitCommand::Push { remote, branch } => {
            run_push(remote, branch)?;
        }
        SupgitCommand::Pull { remote, branch } => {
            run_pull(remote, branch)?;
        }
        SupgitCommand::Sync { remote, branch } => {
            run_sync(remote.as_deref(), branch.as_deref())?;
        }
        SupgitCommand::Commit {
            message,
            all,
            staged,
            unstaged,
            push,
            amend,
            no_verify,
        } => {
            run_commit(message, all, staged, unstaged, push, amend, no_verify)?;
        }
        SupgitCommand::Clone { url, directory } => {
            run_clone(&url, directory.as_deref())?;
        }
        SupgitCommand::Update => {
            run_self_update(None)?;
        }
        SupgitCommand::Alias { dry_run } => {
            run_alias(dry_run)?;
        }
        SupgitCommand::Unalias { dry_run } => {
            run_unalias(dry_run)?;
        }
    }

    Ok(())
}

fn print_explanations() {
    println!("SupGIT simplifies Git for beginners by wrapping each major workflow:");
    println!();
    println!("  init    – initialize a Git repository (runs `git init`).");
    println!("  stage   – add files to the staging area (interactive, or use --all/--tracked).");
    println!("  unstage – remove staged files safely (interactive, or use --all).");
    println!("  status  – show what is staged vs unstaged (`--short` uses `git status -sb`).");
    println!("  log     – view history (`--short` shows compact entries).");
    println!("  diff    – compare working changes (`--staged` shows what will be committed).");
    println!("  branch  – list and checkout branches (interactive); use -c <name> to create, -d <name> to delete a branch.");
    println!("  reset   – discard changes (interactive, or use --all/--staged/--unstaged/--tracked/--untracked).");
    println!(
        "  push    – send commits to your remote (uses Git's defaults unless you pass `--remote`/`--branch`)."
    );
    println!("  pull    – fetch + merge from your remote repository.");
    println!(
        "  commit  – make commits; `--all` stages everything, `--unstaged` stages only modified tracked files, `--push` runs `git push`, `--amend` rewrites the last commit, and `--no-verify` skips hooks."
    );
    println!("  sync    – fetch, pull, and push in one command with graceful error handling.");
    println!("  clone   – clone a repository and automatically change into it.");
    println!("  alias   – add 'git' alias pointing to supgit in your shell config.");
    println!("  unalias – remove the 'git' alias from your shell config.");
    println!("  update  – update supgit to the latest version via cargo.");
}
