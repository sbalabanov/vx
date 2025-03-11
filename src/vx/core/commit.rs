use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::context::Context;
use crate::core::common::{Digest, DigestExt};
use crate::core::tree::Folder;
use crate::global::{DATA_FOLDER, TEMP_FOLDER};
use crate::storage::commit::{self as commitstore, CommitError};

/// Identifier of a commit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CommitID {
    /// Identifier of a branch.
    pub branch: u64,
    /// The sequential number of the commit in a branch.
    pub seq: u64,
}

/// Represents a single commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    // Identifier of a branch.
    pub id: CommitID,
    /// The hash of the file tree root associated with the commit.
    pub treehash: Digest,
    /// The commit message.
    /// TODO: make it a blob?
    pub message: String,
}

impl Commit {
    /// Creates a new Commit instance with the provided values, saves it to the commit store,
    /// and updates the current branch and sequence number.
    pub(crate) fn new(
        context: &Context,
        id: CommitID,
        treehash: Digest,
        message: String,
    ) -> Result<Self, CommitError> {
        let commit = commitstore::new(context, id.branch, id.seq, treehash, message)?;
        commitstore::save_current(context, id)?;
        Ok(commit)
    }

    /// Creates a new commit.
    pub fn make(context: &Context, message: String) -> Result<Self, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        if commit_id.branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        let next_seq = commit_id.seq + 1;
        let treehash: Digest = 0; // TODO: Replace with actual tree hash

        Self::new(
            context,
            CommitID {
                branch: commit_id.branch,
                seq: next_seq,
            },
            treehash,
            message,
        )
    }

    /// Returns a formatted string of the commit information.
    pub fn summary(&self) -> String {
        format!("Commit {}\n{}", self.id.seq, self.message)
    }

    /// Lists all commits for the current branch.
    pub fn list(context: &Context) -> Result<Vec<Self>, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        if commit_id.branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        commitstore::list(context, commit_id.branch)
    }

    /// Retrieves a specific commit by its sequence number.
    pub fn get(context: &Context, seq: u64) -> Result<Self, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        if commit_id.branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        commitstore::get(context, commit_id.branch, seq)
    }

    pub fn get_changed_files(context: &Context) -> Result<Vec<PathBuf>, CommitError> {
        // sergeyb: tried to use walkdir, but it's not working as expected
        // too high level, object creation overhead and can't properly traverse bottom up with filtering

        // get the commit to compare against, so far current commit
        let commit_id = commitstore::get_current(context)?;

        let commit = Self::get(context, commit_id.seq)?;

        // start with root folder's tree and traverse down

        let mut changed_paths = Vec::new();
        let mut level = 1;

        // using 32 as the predicted max depth of the file tree; it is cheap to allocate
        let mut level_states: Vec<LevelState> = Vec::with_capacity(32);

        let mut current_dir = PathBuf::new();
        let mut current_hash = commit.treehash;
        let mut drill = true;

        'vertical: while level > 0 {
            // this loops moves up and down the file tree

            if drill {
                new_level(
                    context,
                    &mut level_states,
                    level,
                    current_dir.clone(),
                    current_hash,
                )?;

                drill = false;
            }

            let mut state = &mut level_states[level - 1];

            'horizontal: loop {
                // this loops moves across directores in the same folder

                // Process folders, compare two sorted lists
                // equal names: advance both iters, proceed down
                // fs < vx: added, advance fs
                // fs > vx: deleted, advance vx
                if state.fs_pos >= state.dirs.len() {
                    // no more dirs to process in filesystem, the remaining ones from vx are deleted from checkout
                    while state.vx_pos < state.vx_folder.folders.len() {
                        let folder = &state.vx_folder.folders[state.vx_pos];
                        changed_paths.push(state.current_dir.join(&folder.name));
                        state.vx_pos += 1;
                    }

                    // drill up
                    process_files(&state, &mut changed_paths)?;
                    clear_state(&mut state);
                    level -= 1;
                    continue 'vertical;
                }

                if state.vx_pos >= state.vx_folder.folders.len() {
                    // no more folder to process in vx, the remaining ones from fs are added to checkout
                    while state.fs_pos < state.dirs.len() {
                        changed_paths.push(state.dirs[state.fs_pos].clone());
                        state.fs_pos += 1;
                    }

                    // drill up
                    process_files(&state, &mut changed_paths)?;
                    clear_state(&mut state);
                    level -= 1;
                    continue 'vertical;
                }

                let fs_dir = &state.dirs[state.fs_pos];
                let vx_dir = &state.vx_folder.folders[state.vx_pos];

                // TODO: figure out if there is more efficient way to compare strings without
                // excessive transformations
                let fs_name = fs_dir.file_name().unwrap().to_str().unwrap();

                match fs_name.cmp(&vx_dir.name) {
                    Ordering::Equal => {
                        // equal names: advance both iters, drill down
                        state.fs_pos += 1;
                        state.vx_pos += 1;

                        // drill down the file tree by breaking into outer loop
                        // keep the current state to return to it later
                        level += 1;
                        current_dir = state.current_dir.join(fs_name);
                        current_hash = vx_dir.blob.contenthash;
                        drill = true;
                        continue 'vertical;
                    }
                    Ordering::Less => {
                        // fs < vx: added, advance fs
                        changed_paths.push(fs_dir.clone());
                        state.fs_pos += 1;
                        continue 'horizontal;
                    }
                    Ordering::Greater => {
                        // fs > vx: deleted, advance vx
                        changed_paths.push(state.current_dir.join(&vx_dir.name));
                        state.vx_pos += 1;
                        continue 'horizontal;
                    }
                }
            }
        }

        Ok(changed_paths)
    }
}

struct LevelState {
    current_dir: PathBuf,
    dirs: Vec<PathBuf>,
    files: Vec<PathBuf>,
    vx_folder: Folder,
    // simple index pointers instead of iterators because Rust ownership rules become hard
    fs_pos: usize,
    vx_pos: usize,
}

fn new_level(
    context: &Context,
    level_states: &mut Vec<LevelState>,
    level: usize,
    current_dir: PathBuf,
    current_hash: Digest,
) -> Result<(), CommitError> {
    // we just went down the file tree, so we need to obtain the current state
    if level_states.len() < level {
        level_states.push(LevelState {
            current_dir,
            dirs: Vec::with_capacity(128),
            files: Vec::with_capacity(128),
            vx_folder: Folder::default(),
            fs_pos: 0,
            vx_pos: 0,
        });
    }

    let state = &mut level_states[level - 1];

    let current_dir_abs = context.checkout_path.join(&state.current_dir);
    let mut entries = std::fs::read_dir(&current_dir_abs)?;

    // Reusing vectors from state object to avoid allocations
    parse_entries(&mut entries, &mut state.dirs, &mut state.files)?;

    // TODO: make it reusing a single connection to the database
    state.vx_folder = Folder::get(context, &current_hash)
        .map_err(|e| CommitError::Other(format!("Failed to get folder: {}", e)))?;

    Ok(())
}

fn parse_entries(
    entries: &mut std::fs::ReadDir,
    dirs: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<(), CommitError> {
    for entry in entries {
        let entry = entry?; // Unwrap the Result<DirEntry, Error>
        let file_name = entry.file_name();

        // Skip .vx and .vxtemp directories
        // TODO: process .gitignore etc
        if file_name == DATA_FOLDER || file_name == TEMP_FOLDER {
            continue;
        }

        if entry.file_type()?.is_dir() {
            dirs.push(entry.path());
        } else {
            files.push(entry.path());
        }
    }

    // Sort directories and files separately by name
    dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    Ok(())
}

/// Clears the state of a level by resetting its vectors and counters
/// This helps reuse the allocated memory instead of reallocating
fn clear_state(state: &mut LevelState) {
    state.dirs.clear();
    state.files.clear();
    state.fs_pos = 0;
    state.vx_pos = 0;
    // Note: we don't clear vx_folder as it will be replaced in new_level
}

/// Process files in the current folder
fn process_files(state: &LevelState, changed_paths: &mut Vec<PathBuf>) -> Result<(), CommitError> {
    let fs_files = &state.files;
    let vx_files = &state.vx_folder.files;

    let mut fs_pos = 0;
    let mut vx_pos = 0;

    // very much a copy of folder processing routine
    // we do not want to unify because of performance
    loop {
        if fs_pos >= fs_files.len() {
            // no more files to process in filesystem, the remaining ones from vx are deleted from checkout
            while vx_pos < vx_files.len() {
                changed_paths.push(state.current_dir.join(&vx_files[vx_pos].name));
                vx_pos += 1;
            }
            break;
        }

        if vx_pos >= vx_files.len() {
            // no more files to process in vx, the remaining ones from fs are added to checkout
            while fs_pos < fs_files.len() {
                changed_paths.push(fs_files[fs_pos].clone());
                fs_pos += 1;
            }
            break;
        }

        // TODO: figure out if there is more efficient way to compare strings without
        // excessive transformations
        let fs_name = fs_files[fs_pos].file_name().unwrap().to_str().unwrap();
        let vx_name = vx_files[vx_pos].name.as_str();

        match fs_name.cmp(vx_name) {
            Ordering::Equal => {
                // equal names: advance both iters and check file contents
                let fs_file_path = &fs_files[fs_pos];

                // Compute hash for the filesystem file
                let (fs_hash, _) = Digest::compute_hash(fs_file_path)?;

                // Get hash from the VX state
                let vx_hash = vx_files[vx_pos].blob.contenthash;

                // If hashes don't match, file has changed
                if fs_hash != vx_hash {
                    changed_paths.push(state.current_dir.join(fs_name));
                }

                fs_pos += 1;
                vx_pos += 1;
            }
            Ordering::Less => {
                // fs < vx: added, advance fs
                changed_paths.push(state.current_dir.join(fs_name));
                fs_pos += 1;
            }
            Ordering::Greater => {
                // fs > vx: deleted, advance vx
                changed_paths.push(state.current_dir.join(vx_name));
                vx_pos += 1;
            }
        }
    }

    Ok(())
}
