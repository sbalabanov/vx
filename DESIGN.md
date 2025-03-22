# vx Design Document

## Introduction

This document outlines the design principles and architecture of vx, a modern version control system designed to address many of Git's limitations while maintaining a familiar workflow. It is intended to help contributors understand the system's design and guide their contributions.

## System Architecture

vx follows an onion architecture pattern, with clear separation between different layers of the application:

```
┌─────────────────────────────────────────────────┐
│                   UI/UX Layer                    │
│  (Command-line interface, user-facing commands)  │
├─────────────────────────────────────────────────┤
│                  Library Layer                   │
│    (Core business logic, domain models, etc.)    │
├─────────────────────────────────────────────────┤
│                 Storage Layer                    │
│  (Persistence, data access, transaction logic)   │
└─────────────────────────────────────────────────┘
```

### UI/UX Layer

Located in `src/bin/vx/`, this layer handles user interaction through the command-line interface. It:
- Parses command-line arguments
- Formats output for users
- Handles user input and errors in a user-friendly way
- Delegates actual work to the library layer

### Library Layer

Located in `src/vx/core/`, this layer contains the business logic of vx. It:
- Defines the domain models (branches, commits, blobs, trees)
- Implements version control algorithms
- Orchestrates operations using the storage layer
- Validates business rules
- Provides a clean API for the UI layer

### Storage Layer

Located in `src/vx/storage/`, this layer handles persistence and data management. It:
- Manages the on-disk representation of vx data
- Implements atomic operations for data manipulation
- Abstracts the actual storage mechanism from the rest of the system
- Handles data serialization/deserialization

## Transaction Model

vx implements an eventual consistency model with the following characteristics:

1. **Atomic Key-Value Storage**: Most components are based on individual atomic key-value storage operations. This means each single write operation is atomic, but a sequence of operations might not be.

2. **Fault Tolerance**: The system is designed to handle hard failures (like SIGKILL) without compromising data integrity. In case of failure, the system might generate some "garbage" (unused or orphaned data), but it will never corrupt the core repository structure.

3. **Optimistic Concurrency**: The system assumes conflicts are rare and optimizes for the non-conflicting case, dealing with conflicts when they occur rather than locking resources preemptively.

4. **No Global Transactions**: Instead of global transactions, vx uses a series of atomic operations that can be retried or cleaned up if necessary.

## Error Handling Philosophy

Error handling in vx follows these principles:

1. **Rich Error Types**: Each module defines its own error types that provide specific information about what went wrong.

2. **Context Preservation**: Errors should flow up through the system with more context added at each level, helping to understand where and why an error occurred.

3. **Structured Errors**: The `thiserror` crate is used to create structured errors that carry information about their source and can be easily converted to user-friendly messages.

4. **Recovery Paths**: When possible, the system should provide ways to recover from errors rather than simply failing.

## Module Dependencies

vx enforces a strict set of rules for module dependencies:

1. **Layered Access**: Upper layers can depend on lower layers, but not vice versa.
   - UI depends on Library
   - Library depends on Storage
   - Storage shouldn't depend on Library or UI

2. **Horizontal Isolation**: Within a layer, modules should respect logical boundaries:
   - Each module from the library layer can access the appropriate module from the storage layer
   - Modules should not access storage modules belonging to different library modules
   - For example, `core::commit` can use `storage::commit` but should not directly use `storage::branch`

3. **Context Passing**: The `Context` object is passed through the system to provide access to workspace paths and other global state without creating global variables.

## Core Data Models

vx is built around these core concepts:

### Repository

The top-level container for version-controlled content, similar to Git. A repository has branches, commits, trees, and blobs.

### Branch

A named pointer to a series of commits. In vx, branches have a clearer relationship to parent branches, creating a true tree structure rather than a DAG.

### Commit

A snapshot of the repository at a point in time. Each commit belongs to a specific branch with a sequential ID, making history navigation more intuitive.

### Tree

Represents the hierarchical structure of files and directories. Uses content-addressing with efficient hashing to track changes.

### Blob

Represents the content of a file, stored and addressed by its hash value.

## Testing Approach

At this prototype stage, vx focuses exclusively on acceptance testing:

1. **Acceptance Testing**: Tests that verify the system works as a whole from the user's perspective.
   - These tests exercise the complete workflow as a user would experience it
   - They validate that features work end-to-end
   - They serve as living documentation of expected system behavior

As the project matures beyond the prototype phase, we may introduce more granular testing approaches.

## Future Design Goals

1. **Improved Remote Repository Support**: Designing efficient protocols for working with remote repositories.

2. **Advanced Partial Checkout Support**: Better solutions for working with large monorepos.

3. **Performance Optimizations**: Continuously improving performance for large repositories.

4. **Enhanced User Interface**: Developing both improved CLI and potential graphical interfaces.

## Contributing Guidelines

When contributing to vx:

1. **Respect the Layered Architecture**: Ensure your changes maintain the separation between UI, library, and storage layers.

2. **Error Handling**: Follow the error handling patterns, adding context and using the appropriate error types.

3. **Modularity**: Keep modules focused and respect the dependency rules.

4. **Performance**: Consider the impact of your changes on performance, especially for large repositories.

5. **Consistency**: Follow the existing coding patterns and naming conventions.

6. **Documentation**: Document your code and update this design document if necessary.

7. **Testing**: Add appropriate acceptance tests for new features. 