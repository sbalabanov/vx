use crate::context::Context;
use crate::core::blob::Blob;
use crate::core::commit::Commit;
use crate::core::common::{Digest, DigestExt};
use crate::global::{DATA_FOLDER, TEMP_FOLDER};
use crate::storage::commit::{self as commitstore};
use crate::storage::tree::{self, TreeError};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;
use xxhash_rust::xxh3::Xxh3;
/// Represents a folder content in a file tree.
/// TODO: add required attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Hash of the folder content, i.e subfolders' and files' hashes.
    pub hash: Digest,
    /// Subfolders in this folder, sorted alphabetically by name.
    pub folders: Vec<File>,
    /// Files in this folder, sorted alphabetically by name.
    pub files: Vec<File>,
}

/// Represents a file in the file tree.
/// TODO: add required attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// Name of the file or folder.
    pub name: String,
    /// Blob containing the file's data or folder's content.
    pub blob: Blob,
}

impl Folder {
    /// Creates a new empty folder with default values.
    pub fn default() -> Self {
        Self {
            hash: Digest::NONE,
            folders: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Creates a new folder with the specified contents.
    pub fn new(context: &Context, folders: Vec<File>, files: Vec<File>) -> Result<Self, TreeError> {
        // Calculate hash based on contents
        let mut hasher = Xxh3::new();

        // Add folder names and hashes to the hash calculation
        for folder in &folders {
            hasher.update(folder.name.as_bytes());
            hasher.update(&folder.blob.contenthash.to_be_bytes());
        }

        // Add file names and hashes to the hash calculation
        for file in &files {
            hasher.update(file.name.as_bytes());
            hasher.update(&file.blob.contenthash.to_be_bytes());
        }

        let folder = Self {
            hash: hasher.digest128(),
            folders,
            files,
        };

        tree::save_folder(context, &folder)?;

        Ok(folder)
    }

    /// Retrieves a folder from the database by its hash.
    pub fn get(context: &Context, hash: &Digest) -> Result<Self, TreeError> {
        tree::get_folder(context, hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {}

pub enum ChangeAction {
    Added,
    Deleted,
    Modified,
}

pub enum ChangeType {
    File,
    Folder,
}

pub struct Change {
    pub action: ChangeAction,
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub contenthash: Digest,
}

impl Tree {
    pub fn get_changed_files(context: &Context) -> Result<Vec<Change>, TreeError> {
        // sergeyb: tried to use walkdir, but it's not working as expected
        // too high level, object creation overhead and can't properly traverse bottom up with filtering

        // get the commit to compare against, so far current commit
        let commit_id = commitstore::get_current(context)
            .map_err(|e| TreeError::Other(format!("Commit error: {:?}", e)))?;

        let commit = Commit::get(context, commit_id)
            .map_err(|e| TreeError::Other(format!("Commit error: {:?}", e)))?;

        traverse_tree_for_changes(context, commit.treehash)
    }

    pub fn create(context: &Context) -> Result<Digest, TreeError> {
        // Return an empty result
        Ok(Digest::NONE)
    }
}

// Walk the file tree and vx tree in parallel, identifying differences.
// There are reasons we are not using recursive algorithm: it would be harder to debug a long stack and
// harder to parallelize.
fn traverse_tree_for_changes(
    context: &Context,
    treehash: Digest,
) -> Result<Vec<Change>, TreeError> {
    // start with root folder's tree and traverse down

    let mut changed_paths = Vec::new();
    let mut level = 1;

    // using 32 as the predicted max depth of the file tree; it is cheap to allocate
    let mut level_states: Vec<LevelState> = Vec::with_capacity(32);

    let mut current_dir = PathBuf::new();
    let mut current_hash = treehash;
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
                    changed_paths.push(Change {
                        action: ChangeAction::Deleted,
                        path: state.current_dir.join(&folder.name),
                        change_type: ChangeType::Folder,
                        contenthash: folder.blob.contenthash,
                    });
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
                    changed_paths.push(Change {
                        action: ChangeAction::Added,
                        path: state.dirs[state.fs_pos].clone(),
                        change_type: ChangeType::Folder,
                        contenthash: Digest::NONE,
                    });
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
                    changed_paths.push(Change {
                        action: ChangeAction::Added,
                        path: fs_dir.clone(),
                        change_type: ChangeType::Folder,
                        contenthash: Digest::NONE,
                    });
                    state.fs_pos += 1;
                    continue 'horizontal;
                }
                Ordering::Greater => {
                    // fs > vx: deleted, advance vx
                    changed_paths.push(Change {
                        action: ChangeAction::Deleted,
                        path: state.current_dir.join(&vx_dir.name),
                        change_type: ChangeType::Folder,
                        contenthash: vx_dir.blob.contenthash,
                    });
                    state.vx_pos += 1;
                    continue 'horizontal;
                }
            }
        }
    }

    Ok(changed_paths)
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
) -> Result<(), TreeError> {
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
    state.vx_folder = Folder::get(context, &current_hash)?;

    Ok(())
}

fn parse_entries(
    entries: &mut std::fs::ReadDir,
    dirs: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<(), TreeError> {
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
fn process_files(state: &LevelState, changed_paths: &mut Vec<Change>) -> Result<(), TreeError> {
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
                changed_paths.push(Change {
                    action: ChangeAction::Deleted,
                    path: state.current_dir.join(&vx_files[vx_pos].name),
                    change_type: ChangeType::File,
                    contenthash: vx_files[vx_pos].blob.contenthash,
                });
                vx_pos += 1;
            }
            break;
        }

        if vx_pos >= vx_files.len() {
            // no more files to process in vx, the remaining ones from fs are added to checkout
            while fs_pos < fs_files.len() {
                let fs_file_path = &fs_files[fs_pos];
                let (fs_hash, _) = Digest::compute_hash(fs_file_path)?;
                changed_paths.push(Change {
                    action: ChangeAction::Added,
                    path: fs_file_path.clone(),
                    change_type: ChangeType::File,
                    contenthash: fs_hash,
                });
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
                    changed_paths.push(Change {
                        action: ChangeAction::Modified,
                        path: state.current_dir.join(fs_name),
                        change_type: ChangeType::File,
                        contenthash: fs_hash,
                    });
                }

                fs_pos += 1;
                vx_pos += 1;
            }
            Ordering::Less => {
                // fs < vx: added, advance fs
                changed_paths.push(Change {
                    action: ChangeAction::Added,
                    path: state.current_dir.join(fs_name),
                    change_type: ChangeType::File,
                    contenthash: Digest::NONE,
                });
                fs_pos += 1;
            }
            Ordering::Greater => {
                // fs > vx: deleted, advance vx
                changed_paths.push(Change {
                    action: ChangeAction::Deleted,
                    path: state.current_dir.join(vx_name),
                    change_type: ChangeType::File,
                    contenthash: vx_files[vx_pos].blob.contenthash,
                });
                vx_pos += 1;
            }
        }
    }

    Ok(())
}
