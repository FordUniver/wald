pub mod bare;
pub mod history;
pub mod shell;
mod worktree;

pub use bare::{clone_bare, fetch_bare, list_branches, open_bare};
pub use history::detect_moves;
pub use shell::worktree_move;
pub use worktree::{add_worktree, list_worktrees, remove_worktree};
