use crate::branch_rules::{self, Scope, Stage};
use crate::default_error::DefaultError;
use crate::versioning_service::{self, VersionRequest};
use crate::{config, git_service, pipelines};
use clap::Args;
use std::error::Error;

#[derive(Args)]
pub(crate) struct VersionCommandArgs {
    /// Version increase or candidate promotion. Inferred from branch rules when omitted.
    #[arg(short, long, env)]
    scope: Option<Scope>,

    /// Version stage. Inferred from branch rules when omitted.
    #[arg(long, env)]
    stage: Option<Stage>,

    /// Promote this exact prerelease tag (official version calculation only).
    #[arg(long, env, value_name = "TAG")]
    candidate: Option<String>,
}

pub(crate) fn run(args: VersionCommandArgs) -> Result<(), Box<dyn Error>> {
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipeline.info();
    let repo_path = config::clone_target_path();
    let (scope, stage) = branch_rules::resolve_policy(
        &pipeline_info.branch_name,
        args.scope,
        args.stage,
        args.candidate.is_some(),
    )?;

    let tag_names = git_service::tag_names(
        &repo_path,
        pipeline_info.force_fetch_tags,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
    )
    .map_err(|e| {
        Box::new(DefaultError {
            message: "Failed to retrieve tags".to_string(),
            source: Some(Box::new(e)),
        })
    })?;

    let versions = versioning_service::calculate(VersionRequest {
        repo_path: &repo_path,
        target_commit: &pipeline.target_commit(),
        short_commit_sha: &pipeline_info.short_commit_sha,
        scope,
        stage,
        candidate: args.candidate.as_deref(),
        tag_names: &tag_names,
    })?;

    println!("UPCOMING_VERSION={}", versions.upcoming_version);
    println!("LAST_VERSION={}", versions.last_version);

    Ok(())
}
