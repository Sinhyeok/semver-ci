use crate::default_error::DefaultError;
use crate::pipelines;
use clap::Args;
use regex::Regex;
use std::error::Error;

#[derive(Args)]
pub(crate) struct ScopeCommandArgs {
    #[arg(long, env, default_value = r"^release/[0-9]+.x.x$")]
    major: String,
    #[arg(
        long,
        env,
        default_value = r"^(develop|feature/.*|release/[0-9]+.[0-9]+.x)$"
    )]
    minor: String,
    #[arg(long, env, default_value = r"^hotfix/[0-9]+.[0-9]+.[0-9]+$")]
    patch: String,
}

pub(crate) fn run(args: ScopeCommandArgs) -> Result<(), Box<dyn Error>> {
    let major_regex = Regex::new(&args.major)?;
    let minor_regex = Regex::new(&args.minor)?;
    let patch_regex = Regex::new(&args.patch)?;

    let pipeline = pipelines::current_pipeline();
    let branch_name = &pipeline.branch_name();

    if major_regex.is_match(branch_name) {
        println!("major")
    } else if minor_regex.is_match(branch_name) {
        println!("minor")
    } else if patch_regex.is_match(branch_name) {
        println!("patch")
    } else {
        return Err(Box::new(DefaultError {
            message: format!("Unknown branch name: {}", branch_name),
            source: None,
        }));
    }

    Ok(())
}
