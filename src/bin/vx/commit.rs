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

pub(super) fn exec(args: &CommitArgs) -> Result<(), String> {
    let context = Context::init().map_err(|err| format!("Error initializing context: {}", err))?;
    match &args.cmd {
        CommitCommands::New { message } => new(&context, message),
        CommitCommands::List => list(&context),
        CommitCommands::Show { id } => show(&context, *id),
    }
}

fn new(context: &Context, message: &str) -> Result<(), String> {
    match Commit::make(context, message.to_string()) {
        Ok(commit) => {
            println!("Created new commit: {} - {}", commit.id.seq, commit.message);
            Ok(())
        }
        Err(e) => Err(format!("Failed to create new commit: {:?}", e)),
    }
}

fn list(context: &Context) -> Result<(), String> {
    match Commit::list(context) {
        Ok(commits) => {
            for commit in commits {
                println!("{}:{}\t{}", commit.id.branch, commit.id.seq, commit.message);
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to list commits: {:?}", e)),
    }
}

fn show(context: &Context, id: Option<u64>) -> Result<(), String> {
    let result = match id {
        Some(commit_id) => Commit::get_from_current_branch(context, commit_id),
        None => Commit::get_current(context),
    };

    match result {
        Ok(commit) => {
            println!(
                "{}:{}\t{}\t{}\n",
                commit.id.branch, commit.id.seq, commit.treehash, commit.message,
            );
            Ok(())
        }
        Err(e) => Err(format!("Failed to show commit: {:?}", e)),
    }
}
