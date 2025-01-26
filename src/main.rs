use std::{error::Error, path::PathBuf};

use clap::{Parser, Subcommand};
use job::run_job;
use runfile::Runfile;

mod job;
mod runfile;
mod step;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List available jobs
    List,

    /// Run a job
    Run(RunArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// Job to run
    job_id: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if !PathBuf::from("uni.toml").exists() {
        eprintln!("No 'uni.toml' found in the current directory");
        std::process::exit(1);
    }

    let runfile = read_runfile()?;

    match args.command {
        Command::List => {
            for (id, job) in runfile.jobs {
                match job.name {
                    Some(name) => println!("{id} - {name}"),
                    None => println!("{id}"),
                }
            }
        }
        Command::Run(args) => {
            let job_id = match args.job_id {
                Some(job_id) => job_id,
                None => runfile.default.clone(),
            };
            run_job(&runfile, job_id.as_str())?
        }
    };

    Ok(())
}

fn read_runfile() -> Result<Runfile, Box<dyn Error>> {
    let content = std::fs::read_to_string("uni.toml")?;

    Ok(toml::from_str(content.as_str())?)
}
