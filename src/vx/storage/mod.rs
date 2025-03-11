pub mod blob;
pub mod branch;
pub mod commit;
pub mod repo;
pub mod tree;

/// The name of the database file.
const BRANCHES_FILE_NAME: &str = "branches.db";
const COMMITS_FILE_NAME: &str = "commits.db";
const REPO_FILE_NAME: &str = "repo.db";
const BLOBS_FOLDER_NAME: &str = "blobs";
