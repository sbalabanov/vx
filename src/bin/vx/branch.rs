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
    New {
        name: String,
    },
    List,
    Show {
        // Optional branch name, if not provided show current branch
        #[arg(default_value = None)]
        name: Option<String>,
    },
}
pub(super) fn exec(args: &BranchArgs) -> Result<(), String> {
    let context = Context::init().map_err(|err| format!("Error initializing context: {}", err))?;
    match &args.cmd {
        BranchCommands::New { name } => new(&context, name),
        BranchCommands::List => list(&context),
        BranchCommands::Show { name } => show(&context, name.clone()),
    }
}

fn new(context: &Context, name: &str) -> Result<(), String> {
    match Branch::new(context, name.to_string()) {
        Ok(branch) => {
            println!("Created new branch: {:?}", branch.name);
            Ok(())
        }
        Err(e) => Err(format!("Failed to create new branch: {:?}", e)),
    }
}

fn list(context: &Context) -> Result<(), String> {
    match Branch::list(context) {
        Ok(branches) => {
            for branch in branches {
                println!(
                    "Branch ID: {}, Name: {}, Version: {}, Head Sequence: {}",
                    branch.id, branch.name, branch.ver, branch.headseq
                );
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to list branches: {:?}", e)),
    }
}

fn show(context: &Context, name: Option<String>) -> Result<(), String> {
    let branch = match name {
        Some(branch_name) => {
            // Show specific branch
            match Branch::get_by_name(context, &branch_name) {
                Ok(branch) => branch,
                Err(e) => return Err(format!("Failed to get branch '{}': {:?}", branch_name, e)),
            }
        }
        None => match Branch::get_current(context) {
            Ok(branch) => branch,
            Err(e) => return Err(format!("Failed to get current branch: {:?}", e)),
        },
    };

    println!("Branch Details:");
    println!("  ID:            {}", branch.id);
    println!("  Name:          {}", branch.name);
    println!("  Version:       {}", branch.ver);
    println!("  Head Sequence: {}", branch.headseq);
    println!("  Parent:        {}", branch.parent);
    println!("  Parent Seq:    {}", branch.parentseq);

    Ok(())
}
