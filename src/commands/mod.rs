mod alias;
mod branch;
mod clone;
mod commit;
mod diff;
mod remote;
mod reset;
mod shelve;
mod stage;
mod sync;
mod unstage;
mod update;
mod worktree;

pub use alias::{run_alias, run_unalias};
pub use branch::{create_branch, delete_branch, run_branch_interactive};
pub use clone::run_clone;
pub use commit::run_commit;
pub use diff::run_diff;
pub use remote::{add_remote, remove_remote, run_remote_interactive, set_remote_url};
pub use reset::run_reset;
pub use shelve::{
    apply_stash, clear_stash, create_stash, get_stashes, run_shelve_interactive, unshelve_stash,
};
pub use stage::stage_targets;
pub use sync::{run_pull, run_push, run_sync};
pub use unstage::restore_stage;
pub use update::{check_and_auto_update, run_self_update};
pub use worktree::{
    create_worktree, get_worktrees, prune_worktrees, remove_worktree, run_worktree_interactive,
};
