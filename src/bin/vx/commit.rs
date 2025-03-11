use clap::{Args, Subcommand};
use vx::context::Context;
use vx::core::commit::{ChangeAction, ChangeType, Commit};

#[derive(Args, Debug)]
pub(super) struct CommitArgs {
    #[command(subcommand)]
    cmd: CommitCommands,
}

#[derive(Debug, Subcommand)]
enum CommitCommands {
    New {
        message: String,
    },
    List,
    Show {
        // TODO: proper commit spec, i.e. branch:seq
        #[arg(default_value = None)]
        id: Option<u64>,
    },
    Files,
}

pub(super) fn exec(args: &CommitArgs) {
    // TODO: Handle errors
    let context = Context::init().unwrap();
    match &args.cmd {
        CommitCommands::New { message } => {
            new(&context, message);
        }
        CommitCommands::List => {
            list(&context);
        }
        CommitCommands::Show { id } => {
            show(&context, *id);
        }
        CommitCommands::Files => {
            files(&context);
        }
    }
}

fn new(context: &Context, message: &str) {
    match Commit::make(context, message.to_string()) {
        Ok(commit) => eprintln!("Created new commit: {} - {}", commit.id.seq, commit.message),
        Err(e) => eprintln!("Failed to create new commit: {:?}", e),
    }
}

fn list(context: &Context) {
    match Commit::list(context) {
        Ok(commits) => {
            for commit in commits {
                eprintln!("{}:{}\t{}", commit.id.branch, commit.id.seq, commit.message);
            }
        }
        Err(e) => eprintln!("Failed to list commits: {:?}", e),
    }
}

fn show(context: &Context, id: Option<u64>) {
    match id {
        Some(commit_id) => match Commit::get_from_current_branch(context, commit_id) {
            Ok(commit) => eprintln!("{}:{}\t{}", commit.id.branch, commit.id.seq, commit.message),
            Err(e) => eprintln!("Failed to show commit: {:?}", e),
        },
        None => {
            // Show current commit when no ID is provided
            match Commit::get_current(context) {
                Ok(commit) => {
                    eprintln!("{}:{}\t{}", commit.id.branch, commit.id.seq, commit.message)
                }
                Err(e) => eprintln!("Failed to show current commit: {:?}", e),
            }
        }
    }
}

fn files(context: &Context) {
    match Commit::get_changed_files(context) {
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
