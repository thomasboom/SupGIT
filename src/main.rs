mod cli;
mod commands;
mod git;
mod status;

use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{Cli, SupgitCommand};
use commands::{
    add_remote, check_and_auto_update, create_branch, delete_branch, remove_remote, restore_stage,
    run_alias, run_branch_interactive, run_clone, run_commit, run_diff, run_pull, run_push,
    run_remote_interactive, run_reset, run_self_update, run_shelve_interactive, run_sync,
    run_tag_interactive, run_unalias, run_worktree_interactive, set_remote_url, stage_targets,
};
use dialoguer::FuzzySelect;
use git::{check_in_repo, run_git, run_git_silent};
use strsim::jaro_winkler;

const COMMANDS: &[(&str, &str)] = &[
    ("init", "Initialize a new Git repository"),
    ("stage", "Stage files for commit"),
    ("unstage", "Unstage files from staging area"),
    ("status", "Show working tree status"),
    ("commit", "Commit staged changes"),
    ("log", "View commit history"),
    ("diff", "Show changes between commits"),
    ("reset", "Reset working tree state"),
    ("branch", "List, create, or delete branches"),
    ("remote", "Manage remote repositories"),
    ("push", "Push commits to remote"),
    ("pull", "Pull commits from remote"),
    ("sync", "Sync with remote (fetch, pull, push)"),
    ("clone", "Clone a repository"),
    ("update", "Update supgit to latest version"),
    ("alias", "Add shell alias"),
    ("unalias", "Remove shell alias"),
    ("shelve", "Shelve changes temporarily"),
    ("worktree", "Manage worktrees"),
    ("tag", "List, create, or delete tags"),
];

fn get_command_names() -> Vec<&'static str> {
    COMMANDS.iter().map(|(name, _)| *name).collect()
}

fn parse_command_from_name(name: &str) -> Option<SupgitCommand> {
    match name {
        "init" => Some(SupgitCommand::Init),
        "stage" => Some(SupgitCommand::Stage {
            targets: vec![],
            all: false,
            tracked: false,
        }),
        "unstage" => Some(SupgitCommand::Unstage {
            targets: vec![],
            all: false,
        }),
        "status" => Some(SupgitCommand::Status { short: false }),
        "commit" => Some(SupgitCommand::Commit {
            message: None,
            all: false,
            staged: false,
            unstaged: false,
            push: false,
            amend: false,
            no_verify: false,
        }),
        "log" => Some(SupgitCommand::Log { short: false }),
        "diff" => Some(SupgitCommand::Diff {
            path: None,
            staged: false,
        }),
        "reset" => Some(SupgitCommand::Reset {
            all: false,
            staged: false,
            unstaged: false,
            tracked: false,
            untracked: false,
        }),
        "branch" => Some(SupgitCommand::Branch {
            create: None,
            delete: None,
        }),
        "push" => Some(SupgitCommand::Push {
            remote: None,
            branch: None,
        }),
        "pull" => Some(SupgitCommand::Pull {
            remote: None,
            branch: None,
        }),
        "sync" => Some(SupgitCommand::Sync {
            remote: None,
            branch: None,
        }),
        "clone" => None,
        "update" => Some(SupgitCommand::Update),
        "alias" => Some(SupgitCommand::Alias {
            dry_run: false,
            git: false,
            sg: false,
        }),
        "unalias" => Some(SupgitCommand::Unalias {
            dry_run: false,
            git: false,
            sg: false,
        }),
        _ => None,
    }
}

fn find_closest_command(input: &str) -> Option<&'static str> {
    get_command_names()
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

fn select_command_interactive() -> Result<SupgitCommand> {
    let items: Vec<String> = COMMANDS
        .iter()
        .map(|(name, desc)| format!("{} – {}", name, desc))
        .collect();

    let selection = FuzzySelect::new()
        .with_prompt("Select a command")
        .items(&items)
        .default(0)
        .interact()
        .context("failed to display command selector")?;

    let (cmd_name, _) = COMMANDS[selection];

    if cmd_name == "clone" {
        bail!("'clone' requires a URL argument; use 'supgit clone <url>'");
    }

    parse_command_from_name(cmd_name).context("failed to parse selected command")
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
            if let Some(unrecognized) = extract_unrecognized_subcommand(&err_string) {
                if let Some(closest) = find_closest_command(unrecognized) {
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
        None => {
            if cli.non_interactive {
                bail!("'supgit' requires a subcommand; use --help to see the available list");
            }
            select_command_interactive()?
        }
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
            | SupgitCommand::Shelve { .. }
            | SupgitCommand::Worktree { .. }
            | SupgitCommand::Tag { .. }
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
        SupgitCommand::Shelve {
            save,
            apply,
            unshelve,
            drop,
            clear,
            list,
        } => {
            if list {
                let stashes = crate::commands::get_stashes()?;
                if stashes.is_empty() {
                    println!("No stashes found.");
                } else {
                    println!("Stashed changes:");
                    for stash in &stashes {
                        println!("  stash@{{{}}}: {}", stash.index, stash.message);
                    }
                }
            } else if let Some(msg) = save {
                crate::commands::create_stash(Some(&msg))?;
            } else if let Some(index) = apply {
                crate::commands::apply_stash(index, false)?;
            } else if let Some(index) = unshelve {
                crate::commands::unshelve_stash(index)?;
            } else if let Some(index) = drop {
                crate::commands::apply_stash(index, true)?;
            } else if clear {
                crate::commands::clear_stash()?;
            } else {
                run_shelve_interactive(cli.non_interactive)?;
            }
        }
        SupgitCommand::Worktree {
            add,
            remove,
            branch,
            new_branch,
            force,
            prune,
            list,
        } => {
            if list {
                let worktrees = crate::commands::get_worktrees()?;
                if worktrees.is_empty() {
                    println!("No worktrees found.");
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
            } else if let Some(path) = add {
                crate::commands::create_worktree(&path, branch.as_deref(), new_branch)?;
            } else if let Some(path) = remove {
                crate::commands::remove_worktree(&path, force)?;
            } else if prune {
                crate::commands::prune_worktrees()?;
            } else {
                run_worktree_interactive(cli.non_interactive)?;
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
        SupgitCommand::Tag {
            create,
            delete,
            push,
            push_all,
            message,
            annotate,
            force,
            list,
        } => {
            if list {
                let tags = crate::commands::get_tags()?;
                if tags.is_empty() {
                    println!("No tags found.");
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
            } else if let Some(name) = create {
                let msg = if annotate {
                    Some(message.as_deref().unwrap_or(""))
                } else {
                    message.as_deref()
                };
                crate::commands::create_tag(&name, msg, force)?;
            } else if let Some(name) = delete {
                crate::commands::delete_tag(&name)?;
            } else if let Some(name) = push {
                crate::commands::push_tag(&name, None)?;
            } else if push_all {
                crate::commands::push_all_tags(None)?;
            } else {
                run_tag_interactive(cli.non_interactive)?;
            }
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
    println!(
        "  shelve  – stash changes (interactive); use --save <msg>, --apply <n>, --unshelve <n>, --drop <n>, --clear, --list"
    );
    println!(
        "  worktree – manage worktrees (interactive); use --add <path> [--branch <name>] [--new-branch], --remove <path> [--force], --prune, --list"
    );
    println!(
        "  tag      – manage tags (interactive); use --create <name> [--message <msg>] [--annotate], --delete <name>, --push <name>, --push-all, --list"
    );
}
