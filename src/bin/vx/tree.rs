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
}

pub(super) fn exec(args: &TreeArgs) {
    // TODO: Handle errors
    let context = Context::init().unwrap();
    match &args.cmd {
        TreeCommands::Status => {
            status(&context);
        }
    }
}

fn status(context: &Context) {
    match Tree::get_changed_files(context) {
        Ok(files) => {
            if files.is_empty() {
                eprintln!("No files changed since current commit");
            } else {
                eprintln!("Files changed since current commit:");
                for file in files {
                    let type_str = match file.change_type {
                        ChangeType::File => "file",
                        ChangeType::Folder => "folder",
                    };
                    let action_str = match file.action {
                        ChangeAction::Added => "added",
                        ChangeAction::Deleted => "deleted",
                        ChangeAction::Modified => "modified",
                    };
                    eprintln!("  {} {} {}", action_str, type_str, file.path.display());
                }
            }
        }
        Err(e) => eprintln!("Failed to list changed files: {:?}", e),
    }
}
