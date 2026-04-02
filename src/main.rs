mod cli;
mod commands;
mod git;
mod status;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, SupgitCommand};
use commands::{
    add_remote, check_and_auto_update, create_branch, delete_branch, remove_remote, restore_stage,
    run_alias, run_branch_interactive, run_clone, run_commit, run_diff, run_pull, run_push,
    run_remote_interactive, run_reset, run_self_update, run_sync, run_unalias, set_remote_url,
    stage_targets,
};
use git::{check_in_repo, run_git, run_git_silent};
use strsim::jaro_winkler;

const COMMANDS: &[&str] = &[
    "init", "stage", "unstage", "status", "commit", "log", "diff", "reset", "branch", "remote",
    "push", "pull", "sync", "clone", "update", "alias", "unalias",
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

    let mut cli = match Cli::try_parse() {
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
                let mut cli = Cli::parse_from(&new_args);
                if let Some(command) = cli.command.take() {
                    return execute_command(command, &cli);
                }
            }
            err.exit();
        }
    };

    if cli.explain {
        print_explanations();
        return Ok(());
    }

    let command = match cli.command.take() {
        Some(command) => command,
        None => bail!("'supgit' requires a subcommand; use --help to see the available list"),
    };

    execute_command(command, &cli)
}

fn execute_command(command: SupgitCommand, cli: &Cli) -> Result<()> {
    if !matches!(
        command,
        SupgitCommand::Init
            | SupgitCommand::Clone { .. }
            | SupgitCommand::Update
            | SupgitCommand::Alias { .. }
            | SupgitCommand::Unalias { .. }
            | SupgitCommand::Remote { .. }
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
        } => stage_targets(&targets, all, tracked, cli.non_interactive)?,
        SupgitCommand::Unstage { targets, all } => {
            restore_stage(&targets, all, cli.non_interactive)?
        }
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
            run_diff(path, staged, cli.non_interactive)?;
        }
        SupgitCommand::Reset {
            all,
            staged,
            unstaged,
            tracked,
            untracked,
        } => run_reset(
            all,
            staged,
            unstaged,
            tracked,
            untracked,
            cli.non_interactive,
        )?,
        SupgitCommand::Branch { create, delete } => {
            if let Some(branch_name) = create {
                create_branch(&branch_name)?;
            } else if let Some(branch_name) = delete {
                delete_branch(&branch_name)?;
            } else {
                run_branch_interactive(cli.non_interactive)?;
            }
        }
        SupgitCommand::Remote {
            add,
            remove,
            set_url,
        } => {
            if let Some(v) = add {
                if v.len() != 2 {
                    bail!("--add requires exactly two arguments: <name> <url>");
                }
                add_remote(&v[0], &v[1])?;
            } else if let Some(name) = remove {
                remove_remote(&name)?;
            } else if let Some(v) = set_url {
                if v.len() != 2 {
                    bail!("--set-url requires exactly two arguments: <name> <url>");
                }
                set_remote_url(&v[0], &v[1])?;
            } else {
                run_remote_interactive(cli.non_interactive)?;
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
            run_commit(
                message,
                all,
                staged,
                unstaged,
                push,
                amend,
                no_verify,
                cli.non_interactive,
            )?;
        }
        SupgitCommand::Clone { url, directory } => {
            run_clone(&url, directory.as_deref())?;
        }
        SupgitCommand::Update => {
            run_self_update(None)?;
        }
        SupgitCommand::Alias { dry_run, git, sg } => {
            run_alias(dry_run, git, sg, cli.non_interactive)?;
        }
        SupgitCommand::Unalias { dry_run, git, sg } => {
            run_unalias(dry_run, git, sg, cli.non_interactive)?;
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
    println!(
        "  branch  – list and checkout branches (interactive); use -c <name> to create, -d <name> to delete a branch."
    );
    println!(
        "  remote  – view, add, remove, or change remote URLs (interactive); use --add <name> <url>, --remove <name>, --set-url <name> <url>"
    );
    println!(
        "  reset   – discard changes (interactive, or use --all/--staged/--unstaged/--tracked/--untracked)."
    );
    println!(
        "  push    – send commits to your remote (uses Git's defaults unless you pass `--remote`/`--branch`)."
    );
    println!("  pull    – fetch + merge from your remote repository.");
    println!(
        "  commit  – make commits; `--all` stages everything, `--unstaged` stages only modified tracked files, `--push` runs `git push`, `--amend` rewrites the last commit, and `--no-verify` skips hooks."
    );
    println!("  sync    – fetch, pull, and push in one command with graceful error handling.");
    println!("  clone   – clone a repository and automatically change into it.");
    println!("  alias   – add alias (--git or --sg, or shows selector).");
    println!("  unalias – remove alias (--git or --sg, or shows selector).");
    println!("  update  – update supgit to the latest version via cargo.");
}
