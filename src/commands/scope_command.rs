use crate::pipelines;
use clap::Args;
use regex::Regex;

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

pub(crate) fn run(args: ScopeCommandArgs) {
    let major_regex = Regex::new(&args.major).unwrap_or_else(|e| panic!("{}", e));
    let minor_regex = Regex::new(&args.minor).unwrap_or_else(|e| panic!("{}", e));
    let patch_regex = Regex::new(&args.patch).unwrap_or_else(|e| panic!("{}", e));

    let pipeline_info = pipelines::pipeline_info(false);
    let branch_name = pipeline_info.branch_name.as_str();

    if major_regex.is_match(branch_name) {
        println!("major")
    } else if minor_regex.is_match(branch_name) {
        println!("minor")
    } else if patch_regex.is_match(branch_name) {
        println!("patch")
    } else {
        panic!("Unknown branch name: {}", branch_name)
    }
}
