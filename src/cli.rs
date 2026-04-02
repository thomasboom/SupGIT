use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "supgit",
    about = "Blazing fast wrapper for Git with simplified workflows",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[arg(short = 'n', long, global = true)]
    pub non_interactive: bool,

    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(long, global = true)]
    pub explain: bool,

    #[command(subcommand)]
    pub command: Option<SupgitCommand>,
}

#[derive(Subcommand)]
pub enum SupgitCommand {
    Init,
    Stage {
        #[arg(value_name = "PATH")]
        targets: Vec<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        tracked: bool,
    },
    Unstage {
        #[arg(value_name = "PATH")]
        targets: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    Status {
        #[arg(long)]
        short: bool,
    },
    Commit {
        #[arg(short, long, value_name = "MSG")]
        message: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        staged: bool,
        #[arg(long)]
        unstaged: bool,
        #[arg(long)]
        push: bool,
        #[arg(long)]
        amend: bool,
        #[arg(long)]
        no_verify: bool,
    },
    Log {
        #[arg(long)]
        short: bool,
    },
    Diff {
        path: Option<String>,
        #[arg(long)]
        staged: bool,
    },
    Reset {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        staged: bool,
        #[arg(long)]
        unstaged: bool,
        #[arg(long)]
        tracked: bool,
        #[arg(long)]
        untracked: bool,
    },
    Branch {
        #[arg(short, long)]
        create: Option<String>,
        #[arg(short, long)]
        delete: Option<String>,
    },
    Push {
        remote: Option<String>,
        branch: Option<String>,
    },
    Pull {
        remote: Option<String>,
        branch: Option<String>,
    },
    Sync {
        remote: Option<String>,
        branch: Option<String>,
    },
    Clone {
        #[arg(value_name = "URL")]
        url: String,
        #[arg(value_name = "DIR")]
        directory: Option<String>,
    },
    Update,
    Alias {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        git: bool,
        #[arg(long)]
        sg: bool,
    },
    Unalias {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        git: bool,
        #[arg(long)]
        sg: bool,
    },
    Remote {
        #[arg(short, long, value_name = "NAME", num_args = 2)]
        add: Option<Vec<String>>,
        #[arg(short, long, value_name = "NAME")]
        remove: Option<String>,
        #[arg(long, value_name = "NAME", num_args = 2)]
        set_url: Option<Vec<String>>,
    },
}
