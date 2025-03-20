use clap::{Args, Subcommand};
use vx::context::Context;
use vx::core::tree::{ChangeAction, ChangeType, Tree};

#[derive(Args, Debug)]
pub(super) struct TreeArgs {
    #[command(subcommand)]
    cmd: TreeCommands,
}

#[derive(Debug, Subcommand)]
enum TreeCommands {
    Status,
    Checkout {
        /// The commit ID to checkout
        commit_id: String,
    },
}

pub(super) fn exec(args: &TreeArgs) -> Result<(), String> {
    let context = Context::init().map_err(|err| format!("Error initializing context: {}", err))?;
    match &args.cmd {
        TreeCommands::Status => status(&context),
        TreeCommands::Checkout { commit_id } => checkout(&context, commit_id),
    }
}

fn status(context: &Context) -> Result<(), String> {
    match Tree::get_changed_files(context) {
        Ok(changes) => {
            if changes.is_empty() {
                eprintln!("No files changed since current commit");
            } else {
                eprintln!("Files changed since current commit:");
                for change in changes {
                    let type_str = match change.change_type {
                        ChangeType::File => "file",
                        ChangeType::Folder => "folder",
                    };
                    let action_str = match change.action {
                        ChangeAction::Added => "added",
                        ChangeAction::Deleted => "deleted",
                        ChangeAction::Modified => "modified",
                    };
                    eprintln!("  {} {} {}", action_str, type_str, change.path.display());
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to list changed files: {:?}", e)),
    }
}

fn checkout(context: &Context, commit_id: &str) -> Result<(), String> {
    match Tree::checkout(context, commit_id) {
        Ok(()) => {
            eprintln!("Successfully checked out commit: {}", commit_id);
            Ok(())
        }
        Err(e) => Err(format!("Failed to checkout commit: {:?}", e)),
    }
}
