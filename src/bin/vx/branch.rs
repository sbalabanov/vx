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
}
pub(super) fn exec(args: &BranchArgs) -> Result<(), String> {
    let context = Context::init().map_err(|err| format!("Error initializing context: {}", err))?;
    match &args.cmd {
        BranchCommands::New { name } => new(&context, name),
        BranchCommands::List => list(&context),
    }
}

fn new(context: &Context, name: &str) -> Result<(), String> {
    match Branch::new(context, name.to_string()) {
        Ok(branch) => {
            eprintln!("Created new branch: {:?}", branch.name);
            Ok(())
        }
        Err(e) => Err(format!("Failed to create new branch: {:?}", e)),
    }
}

fn list(context: &Context) -> Result<(), String> {
    match Branch::list(context) {
        Ok(branches) => {
            for branch in branches {
                eprintln!("Branch ID: {}, Name: {}", branch.id, branch.name);
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to list branches: {:?}", e)),
    }
}
