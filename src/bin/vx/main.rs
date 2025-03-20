use clap::{Parser, Subcommand};

mod branch;
mod commit;
mod repo;
mod tree;

#[derive(Parser, Debug)]
#[command(
    author = "Sergey Balabanov",
    version = "0.1.0",
    about = "Simple and powerful version control system"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Branch(branch::BranchArgs),
    Commit(commit::CommitArgs),
    Repo(repo::RepoArgs),
    Tree(tree::TreeArgs),
}

fn main() {
    let cli = Cli::parse();
    let result = match &cli.cmd {
        Commands::Branch(args) => branch::exec(args),
        Commands::Commit(args) => commit::exec(args),
        Commands::Repo(args) => repo::exec(args),
        Commands::Tree(args) => tree::exec(args),
    };

    if let Err(err) = result {
        eprintln!("{}", err);

        // TODO: binary protocol, i.e. exit code based on error type.
        std::process::exit(1);
    }
}
