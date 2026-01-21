pub mod baum;
mod discovery;
pub mod gitignore;
mod path_safety;

pub use baum::{create_baum, is_baum};
pub use discovery::{find_workspace_root, Workspace};
pub use gitignore::ensure_gitignore_section;
pub use path_safety::validate_workspace_path;
