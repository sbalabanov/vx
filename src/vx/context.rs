use crate::global::DATA_FOLDER;
use std::path::PathBuf;

/// Represents the context of the version control system.
#[derive(Debug, Clone)]
pub struct Context {
    /// Path to the workspace directory.
    pub workspace_path: PathBuf,
}

impl Context {
    /// Creates a new Context with the given workspace path.
    fn new(workspace_path: PathBuf) -> Self {
        Context { workspace_path }
    }
    /// Searches the current working directory and upwards for a folder named `.vx`.
    /// If found, returns a Context object initialized with the path to this folder.
    /// Otherwise, returns an error.
    pub fn init() -> Result<Self, std::io::Error> {
        let mut current_dir = std::env::current_dir()?;

        loop {
            let vx_path = current_dir.join(DATA_FOLDER);
            if vx_path.is_dir() {
                return Ok(Context::new(current_dir));
            }

            if !current_dir.pop() {
                break;
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "No {} directory found in current directory or any parent directories",
                DATA_FOLDER
            ),
        ))
    }
}
