pub mod bare;
pub mod history;
pub mod shell;
mod worktree;

pub use bare::{
    clone_bare, fetch_bare, fetch_full, gc, is_partial_clone, list_branches, open_bare,
    CloneOptions,
};
pub use history::detect_moves;
pub use shell::worktree_move;
pub use worktree::{
    add_worktree, add_worktree_with_tracking, add_worktree_with_tracking_mode, check_branch_exists,
    delete_branch, has_unpushed_commits, list_wald_branches, list_worktrees, remove_worktree,
    BranchMode,
};
