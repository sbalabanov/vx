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
    New { message: String },
    List,
    Show { id: u64 },
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
    match Commit::new(context, message.to_string()) {
        Ok(commit) => eprintln!("Created new commit: {} - {}", commit.seq, commit.message),
        Err(e) => eprintln!("Failed to create new commit: {:?}", e),
    }
}

fn list(context: &Context) {
    match Commit::list(context) {
        Ok(commits) => {
            for commit in commits {
                eprintln!("Commit Seq: {}, Message: {}", commit.seq, commit.message);
            }
        }
        Err(e) => eprintln!("Failed to list commits: {:?}", e),
    }
}

fn show(context: &Context, id: u64) {
    match Commit::get(context, id) {
        Ok(commit) => eprintln!("Commit Seq: {}\nMessage: {}", commit.seq, commit.message),
        Err(e) => eprintln!("Failed to show commit: {:?}", e),
    }
}
