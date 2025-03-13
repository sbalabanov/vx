use crate::context::Context;
use crate::core::blob::Blob;
use crate::core::commit::Commit;
use crate::core::common::{Digest, DigestExt};
use crate::global::{DATA_FOLDER, TEMP_FOLDER};
use crate::storage::commit::{self as commitstore};
use crate::storage::tree::{self as treestore, TreeError};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::Xxh3;
/// Represents a folder content in a file tree.
/// TODO: add required attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    /// Hash of the folder content, i.e subfolders' and files' hashes.
    pub hash: Digest,
    /// Subfolders in this folder, sorted alphabetically by name.
    pub folders: Vec<Folder>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents a folder in the file tree.
/// Contains the folder name and a hash of its contents.
pub struct Folder {
    /// Name of the folder.
    pub name: String,
    /// Hash of the folder's tree, calculated from its files and subfolders.
    pub hash: Digest,
}

impl Tree {
    /// Creates a new empty folder with default values.
    pub fn default() -> Self {
        Self {
            hash: Digest::NONE,
            folders: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Creates a new empty tree and saves it to the database.
    pub fn create_empty(context: &Context) -> Result<Self, TreeError> {
        let db = treestore::open(context)?;
        let tree = new_tree(&db, Vec::new(), Vec::new())?;
        treestore::save(&db, &tree)?;
        db.flush()?;
        Ok(tree)
    }

    pub fn get_changed_files(context: &Context) -> Result<Vec<Change>, TreeError> {
        // sergeyb: tried to use walkdir, but it's not working as expected
        // too high level, object creation overhead and can't properly traverse bottom up with filtering

        // get the commit to compare against, so far current commit
        let commit_id = commitstore::get_current(context)
            .map_err(|e| TreeError::Other(format!("Commit error: {:?}", e)))?;

        let commit = Commit::get(context, commit_id)
            .map_err(|e| TreeError::Other(format!("Commit error: {:?}", e)))?;

        let db = treestore::open(context)?;
        traverse_tree_for_changes(&context, &db, commit.treehash)
    }

    /// Creates a new tree from the current directory recursively.
    pub fn create(context: &Context) -> Result<Digest, TreeError> {
        let db = treestore::open(context)?;
        persist_tree(&context, &db, Path::new(""))
    }
}

impl File {
    pub fn from_path(context: &Context, name: String, path: &Path) -> Result<Self, TreeError> {
        let blob = Blob::from_file_and_store(context, path)
            .map_err(|e| TreeError::Other(format!("Blob error for path {:?}: {:?}", path, e)))?;
        let file = Self { name: name, blob };
        Ok(file)
    }
}

#[derive(Debug, Clone)]
pub enum ChangeAction {
    Added,
    Deleted,
    Modified,
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    File,
    Folder,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub action: ChangeAction,
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub contenthash: Digest,
}

/// Creates a new tree with the specified contents.
fn new_tree(db: &Db, folders: Vec<Folder>, files: Vec<File>) -> Result<Tree, TreeError> {
    // Calculate hash based on contents
    let mut hasher = Xxh3::new();

    // Add folder names and hashes to the hash calculation
    for folder in &folders {
        hasher.update(folder.name.as_bytes());
        hasher.update(&folder.hash.to_be_bytes());
    }

    // Add file names and hashes to the hash calculation
    for file in &files {
        hasher.update(file.name.as_bytes());
        hasher.update(&file.blob.contenthash.to_be_bytes());
    }

    let tree = Tree {
        hash: hasher.digest128(),
        folders,
        files,
    };

    treestore::save(db, &tree)?;

    Ok(tree)
}

// Walk the file tree and vx tree in parallel, identifying differences.
// There are reasons we are not using recursive algorithm: it would be harder to debug a long stack and
// harder to parallelize.
fn traverse_tree_for_changes(
    context: &Context,
    db: &Db,
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
                db,
                &mut level_states,
                level,
                current_dir.clone(),
                current_hash,
            )?;

            drill = false;
        }

        let state = &mut level_states[level - 1];

        'horizontal: loop {
            // this loops moves across directores in the same folder

            // Process folders, compare two sorted lists
            // equal names: advance both iters, proceed down
            // fs < vx: added, advance fs
            // fs > vx: deleted, advance vx
            if state.fs_pos >= state.dirs.len() {
                // no more dirs to process in filesystem, the remaining ones from vx are deleted from checkout
                while state.vx_pos < state.vx_tree.folders.len() {
                    let folder = &state.vx_tree.folders[state.vx_pos];
                    changed_paths.push(Change {
                        action: ChangeAction::Deleted,
                        path: state.current_dir.join(&folder.name),
                        change_type: ChangeType::Folder,
                        contenthash: folder.hash,
                    });
                    state.vx_pos += 1;
                }

                process_files(&context, &state, &mut changed_paths)?;

                // drill up
                level -= 1;
                continue 'vertical;
            }

            if state.vx_pos >= state.vx_tree.folders.len() {
                // no more folder to process in vx, the remaining ones from fs are added to checkout
                while state.fs_pos < state.dirs.len() {
                    changed_paths.push(Change {
                        action: ChangeAction::Added,
                        path: state.current_dir.join(&state.dirs[state.fs_pos]),
                        change_type: ChangeType::Folder,
                        contenthash: Digest::NONE,
                    });
                    state.fs_pos += 1;
                }

                process_files(&context, &state, &mut changed_paths)?;

                // drill up
                level -= 1;
                continue 'vertical;
            }

            let fs_name = &state.dirs[state.fs_pos];
            let vx_dir = &state.vx_tree.folders[state.vx_pos];

            match fs_name.cmp(&vx_dir.name) {
                Ordering::Equal => {
                    // equal names: advance both iters, drill down
                    state.fs_pos += 1;
                    state.vx_pos += 1;

                    // drill down the file tree by breaking into outer loop
                    // keep the current state to return to it later
                    level += 1;
                    current_dir = state.current_dir.join(fs_name);
                    current_hash = vx_dir.hash;
                    drill = true;
                    continue 'vertical;
                }
                Ordering::Less => {
                    // fs < vx: added, advance fs
                    changed_paths.push(Change {
                        action: ChangeAction::Added,
                        path: state.current_dir.join(fs_name),
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
                        contenthash: vx_dir.hash,
                    });
                    state.vx_pos += 1;
                    continue 'horizontal;
                }
            }
        }
    }

    Ok(changed_paths)
}

#[derive(Debug, Clone)]
struct LevelState {
    current_dir: PathBuf,
    dirs: Vec<String>,
    files: Vec<String>,
    vx_tree: Tree,
    // simple index pointers instead of iterators because Rust ownership rules become hard
    fs_pos: usize,
    vx_pos: usize,
}

fn new_level(
    context: &Context,
    db: &Db,
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
            vx_tree: Tree::default(),
            fs_pos: 0,
            vx_pos: 0,
        });
    } else {
        let state = &mut level_states[level - 1];
        state.current_dir = current_dir;
        state.dirs.clear();
        state.files.clear();
        state.fs_pos = 0;
        state.vx_pos = 0;
        // state.vx_tree will be setup later
    }

    let state = &mut level_states[level - 1];

    let current_dir_abs = context.checkout_path.join(&state.current_dir);
    let mut entries = std::fs::read_dir(&current_dir_abs)?;

    // Reusing vectors from state object to avoid allocations
    parse_entries(&mut entries, &mut state.dirs, &mut state.files)?;

    state.vx_tree = treestore::get(db, current_hash)?;

    Ok(())
}

fn parse_entries(
    entries: &mut std::fs::ReadDir,
    dirs: &mut Vec<String>,
    files: &mut Vec<String>,
) -> Result<(), TreeError> {
    for entry in entries {
        let entry = entry?; // Unwrap the Result<DirEntry, Error>
        let file_name = entry.file_name();

        // Skip .vx and .vxtemp directories
        // TODO: process .gitignore etc
        if file_name == DATA_FOLDER || file_name == TEMP_FOLDER {
            continue;
        }

        let ftype = entry.file_type()?;
        if ftype.is_dir() {
            dirs.push(file_name.into_string().unwrap());
        } else {
            if ftype.is_symlink() {
                // Skip symlinks and return an error
                return Err(TreeError::Other(format!(
                    "Symlinks are not supported as of yet: {:?}",
                    entry.path()
                )));
            }
            files.push(file_name.into_string().unwrap());
        }
    }

    dirs.sort();
    files.sort();

    Ok(())
}

/// Process files in the current folder
fn process_files(
    context: &Context,
    state: &LevelState,
    changed_paths: &mut Vec<Change>,
) -> Result<(), TreeError> {
    let fs_files = &state.files;
    let vx_files = &state.vx_tree.files;

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
                let fs_file_name = &fs_files[fs_pos];
                let fs_file_path = state.current_dir.join(fs_file_name);

                let (fs_hash, _) =
                    Digest::compute_hash(&context.checkout_path.join(&fs_file_path))?;
                changed_paths.push(Change {
                    action: ChangeAction::Added,
                    path: fs_file_path,
                    change_type: ChangeType::File,
                    contenthash: fs_hash,
                });
                fs_pos += 1;
            }
            break;
        }

        let fs_name = &fs_files[fs_pos];
        let vx_name = &vx_files[vx_pos].name;

        match fs_name.cmp(vx_name) {
            Ordering::Equal => {
                // equal names: advance both iters and check file contents
                let fs_file_name = &fs_files[fs_pos];
                let fs_file_path = state.current_dir.join(fs_file_name);

                // Compute hash for the filesystem file
                let (fs_hash, _) =
                    Digest::compute_hash(&context.checkout_path.join(&fs_file_path))?;

                // Get hash from the VX state
                let vx_hash = vx_files[vx_pos].blob.contenthash;

                // If hashes don't match, file has changed
                if fs_hash != vx_hash {
                    changed_paths.push(Change {
                        action: ChangeAction::Modified,
                        path: fs_file_path,
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

// (UNOPTIMIZED) Creates a tree from a directory, saving entities to storage on the go
fn persist_tree(context: &Context, db: &Db, path: &Path) -> Result<Digest, TreeError> {
    // Unlike in get changes, here we go with the recursive algorithm. It will likely be
    // rewritten anyways so going with it for the sake of time.

    // Get the absolute path to work with
    let abs_path = context.checkout_path.join(path);

    // If it's a directory, process its contents
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    let mut vx_folders = Vec::new();
    let mut vx_files = Vec::new();

    // Read directory entries
    let mut entries = std::fs::read_dir(&abs_path)?;

    // parse entries
    parse_entries(&mut entries, &mut dirs, &mut files)?;

    for dir in dirs.into_iter() {
        let treehash = persist_tree(context, db, &path.join(&dir))?;
        // Add folder to vx_folders with the hash
        vx_folders.push(Folder {
            name: dir,
            hash: treehash,
        });
    }

    for file in files.into_iter() {
        let file_path = abs_path.join(&file);
        vx_files.push(File::from_path(&context, file, &file_path)?);
    }

    let tree = new_tree(db, vx_folders, vx_files)?;

    Ok(tree.hash)
}
