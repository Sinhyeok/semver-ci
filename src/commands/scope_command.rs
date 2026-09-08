use crate::branch_rules::{
    ScopePatterns, MAJOR_PATTERN, MINOR_PATTERN, PATCH_PATTERN, STABLE_PATTERN,
};
use crate::default_error::Result;
use crate::pipelines;
use clap::Args;

#[derive(Args)]
pub(crate) struct ScopeCommandArgs {
    #[arg(long, env, default_value = MAJOR_PATTERN)]
    major: String,

    #[arg(long, env, default_value = MINOR_PATTERN)]
    minor: String,

    #[arg(long, env, default_value = PATCH_PATTERN)]
    patch: String,

    #[arg(long, env, default_value = STABLE_PATTERN)]
    release: String,
}

pub(crate) fn run(args: ScopeCommandArgs) -> Result<()> {
    let pipeline = pipelines::current_pipeline()?;
    let patterns = ScopePatterns {
        major: args.major,
        minor: args.minor,
        patch: args.patch,
        release: args.release,
    };
    println!("{}", patterns.resolve(&pipeline.branch_name()?)?.as_str());
    Ok(())
}
