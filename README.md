# vx
Simple and powerful version control system

**⚠️ WORK IN PROGRESS ⚠️**

This project is currently under active development and is not yet ready for production use.

## Inspiration

Git has revolutionized version control systems, but it comes with several limitations and challenges:

1. **Complex mental model**: Git's internal model based on directed acyclic graphs (DAGs) can be difficult to understand, with concepts like detached HEAD states, rebasing vs merging, and complex branching strategies.

2. **Inefficient storage for large files**: Git stores the entire history of each file, making it inefficient for large binary files or repositories with extensive history.

3. **Limited partial checkout support**: Git offers sparse checkouts and partial clones, but these features are not well-integrated or user-friendly, making them challenging to use effectively with large monorepos.

4. **Steep learning curve**: Git's command interface is often unintuitive, with many commands having inconsistent syntax and behavior.

5. **Performance issues with large repositories**: Git can become slow when dealing with repositories that have extensive history or large numbers of files.

6. **Limited built-in large file support**: While Git LFS exists, it's not part of the core Git experience and adds complexity.

## How vx Improves Version Control

vx is a modern version control system designed to address many of Git's limitations while maintaining a familiar workflow. Here's how vx improves the version control experience:

### Simplified Branch Model

vx uses a straightforward branch model with intuitive semantics:

- Branches are identified by unique names, with uniqueness guaranteed across all replicas of the repository (no distinction between "local" and "remote" branches)
- Each branch has a clear relationship to its parent branch
- vx only supports rebasing (not merging), which creates a true tree structure rather than a DAG, simplifying the mental model
- Rebasing is handled more gracefully with improved change tracking
- The foundational branch (typically "main") serves as the base for other branches
- Each commit always belongs to a single branch, eliminating the concept of "dangling commits"
- Clear history visualization with straightforward parent-child relationships

### Content-Addressed Storage

vx implements an efficient storage mechanism:

- Files are stored as content-addressed blobs using high-performance XXH3 hashing
- Directory trees are recursively hashed, providing integrity verification
- The storage architecture allows for future optimizations like deduplication

### Performance-Oriented Design

vx is designed with performance in mind:

- Parallel processing for tree creation and traversal using the Rayon library
- Efficient database storage with Sled for fast lookups and persistence
- Optimized change detection algorithms to identify modifications
- Extensible architecture that allows for custom high-performance backends

### Simplified Command Interface

vx provides a straightforward command interface:

- Consistent command patterns with intuitive subcommands
- Clear error messages that explain what went wrong and how to fix it
- Reduced command complexity while maintaining power and flexibility

## Key Components

### Tree Structure

The tree structure in vx represents the file hierarchy with several optimizations:

- Directories and files are stored separately for efficient traversal
- Metadata like file counts and sizes are cached at each level
- Each tree node contains information about its children, hashed for integrity

### Commit System

Commits in vx are designed to be more intuitive:

- Each commit belongs to a specific branch with a sequential ID, making discovery and finding common ancestors easier
- The commit history maintains a clear lineage

### Branch Management

The branch system in vx provides:

- Simple creation of new branches from the current state
- Clean relationship tracking between parent and child branches
- Efficient navigation between branches

### Extensible Architecture

vx's model is designed to be open for modifications:

- Modular components that can be replaced or extended
- Support for developing custom high-performance backends
- Clear interfaces between system components for easier customization

## Future Development

vx is actively being developed with plans for:

- Improved merging and conflict resolution
- Remote repository support
- Performance optimizations for large repositories
- Graphical user interface

## Contributing

vx is open to contributions. Stay tuned for contribution guidelines as the project matures.

## License

See the LICENSE file for details.
