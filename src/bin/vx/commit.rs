use clap::{Args, Subcommand};
use vx::context::Context;
use vx::core::commit::Commit;

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
    let result = if let Some(commit_id) = id {
        Commit::get_from_current_branch(context, commit_id)
    } else {
        // Show current commit when no ID is provided
        Commit::get_current(context)
    };

    if let Ok(commit) = result {
        eprintln!(
            "{}:{}\t{}\t{}\n",
            commit.id.branch, commit.id.seq, commit.treehash, commit.message,
        );
    } else if let Err(e) = result {
        eprintln!("Failed to show commit: {:?}", e);
    }
}
