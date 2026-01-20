pub mod baum;
mod discovery;
pub mod gitignore;

pub use baum::{create_baum, is_baum};
pub use discovery::{find_workspace_root, Workspace};
pub use gitignore::ensure_gitignore_section;
