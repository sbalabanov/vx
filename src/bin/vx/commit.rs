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
    List {
        // Optional branch name to list commits from
        #[arg(default_value = None)]
        branch: Option<String>,
    },
    Show {
        // Commit specification in format "branch_name:seq" or just "seq" or "branch_name"
        #[arg(default_value = None)]
        spec: Option<String>,
    },
    Amend {
        message: Option<String>,
    },
}

pub(super) fn exec(args: &CommitArgs) -> Result<(), String> {
    let context = Context::init().map_err(|err| format!("Error initializing context: {}", err))?;
    match &args.cmd {
        CommitCommands::New { message } => new(&context, message.clone()),
        CommitCommands::List { branch } => list(&context, branch.clone()),
        CommitCommands::Show { spec } => show(&context, spec.clone()),
        CommitCommands::Amend { message } => amend(&context, message.clone()),
    }
}

fn new(context: &Context, message: String) -> Result<(), String> {
    match Commit::new(context, message) {
        Ok(commit) => {
            println!("Created new commit: {} - {}", commit.id.seq, commit.message);
            Ok(())
        }
        Err(e) => Err(format!("Failed to create new commit: {:?}", e)),
    }
}

fn list(context: &Context, branch: Option<String>) -> Result<(), String> {
    let commits = match branch {
        Some(branch_name) => Commit::list_by_branch(context, &branch_name).map_err(|e| {
            format!(
                "Failed to list commits for branch '{}': {:?}",
                branch_name, e
            )
        })?,
        None => Commit::list(context).map_err(|e| format!("Failed to list commits: {:?}", e))?,
    };

    for commit in commits {
        println!(
            "{}:{}\tv{}\t{}",
            commit.id.branch, commit.id.seq, commit.ver, commit.message
        );
    }
    Ok(())
}

fn show(context: &Context, spec: Option<String>) -> Result<(), String> {
    let result = match spec {
        Some(commit_spec) => Commit::get_by_spec(context, &commit_spec),
        None => Commit::get_current(context),
    };

    match result {
        Ok(commit) => {
            println!(
                "Branch: {}\nSequence: {}\nHash: {}\nTree Hash: {}\nVersion: {}\nMessage: {}\n",
                commit.id.branch,
                commit.id.seq,
                commit.hash,
                commit.treehash,
                commit.ver,
                commit.message,
            );
            Ok(())
        }
        Err(e) => Err(format!("Failed to show commit: {:?}", e)),
    }
}

fn amend(context: &Context, message: Option<String>) -> Result<(), String> {
    match Commit::amend(context, message) {
        Ok(commit) => {
            println!("Amended commit: {} - {}", commit.id.seq, commit.message);
            Ok(())
        }
        Err(e) => Err(format!("Failed to amend commit: {:?}", e)),
    }
}
