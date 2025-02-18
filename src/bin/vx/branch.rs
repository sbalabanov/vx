use clap::{Args, Subcommand};
use vx::context::Context;
use vx::core::branch::Branch;

#[derive(Args, Debug)]
pub(super) struct BranchArgs {
    #[command(subcommand)]
    cmd: BranchCommands,
}

#[derive(Debug, Subcommand)]
enum BranchCommands {
    New { name: String },
    List,
    Delete { name: String },
}
pub(super) fn exec(args: &BranchArgs) {
    // TODO: Handle errors
    let context = Context::init().unwrap();
    match &args.cmd {
        BranchCommands::New { name } => {
            new(&context, name);
        }
        BranchCommands::List => {
            list(&context);
        }
        BranchCommands::Delete { name } => {
            delete(&context, name);
        }
    }
}

fn new(context: &Context, name: &str) {
    // TODO: current commit number
    match Branch::new(context, name.to_string(), 0, 0, 0) {
        Ok(branch) => eprintln!("Created new branch: {:?}", branch.name),
        Err(e) => eprintln!("Failed to create new branch: {:?}", e),
    }
}

fn list(context: &Context) {
    match Branch::list(context) {
        Ok(branches) => {
            for branch in branches {
                eprintln!("Branch ID: {}, Name: {}", branch.id, branch.name);
            }
        }
        Err(e) => eprintln!("Failed to list branches: {:?}", e),
    }
}

fn delete(context: &Context, name: &str) {
    match Branch::delete(context, name) {
        Ok(_) => eprintln!("Deleted branch '{}'", name),
        Err(e) => eprintln!("Failed to delete branch: {:?}", e),
    }
}
