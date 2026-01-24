pub mod baum;
mod discovery;
pub mod gitignore;
mod path_safety;

pub use baum::{create_baum, is_baum, save_baum_with_id};
pub use discovery::{Workspace, collect_baum_ids, find_all_baums, find_workspace_root};
pub use gitignore::ensure_gitignore_section;
pub use path_safety::validate_workspace_path;
