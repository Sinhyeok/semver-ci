use crate::branch_rules::{
    self, ScopePatterns, StagePatterns, DEV_PATTERN, MAJOR_PATTERN, MINOR_PATTERN, PATCH_PATTERN,
    RC_PATTERN, STABLE_PATTERN,
};
use crate::errors::{messages, Result, ResultExt};
use crate::models::{Scope, Stage};
use crate::versioning::{self, VersionRequest};
use crate::{config, git_service, pipelines};
use clap::Args;

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

    /// Exact official target version. Must agree with a version-bearing branch.
    #[arg(long, env, value_name = "VERSION")]
    target: Option<String>,
}

pub(crate) fn run(args: VersionCommandArgs) -> Result<()> {
    let pipeline = pipelines::current_pipeline()?;
    pipeline.init()?;
    let pipeline_info = pipeline.info()?;
    let repo_path = config::clone_target_path()?;
    let (scope, stage) = resolve_policy(&pipeline_info.branch_name, &args)?;
    let target =
        branch_rules::resolve_target(&pipeline_info.branch_name, args.target.as_deref(), scope)?;

    let tag_names = git_service::tag_names(
        &repo_path,
        pipeline_info.force_fetch_tags,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
    )
    .context(messages::RETRIEVE_TAGS)?;

    let versions = versioning::calculate(VersionRequest {
        repo_path: &repo_path,
        target_commit: &pipeline.target_commit()?,
        short_commit_sha: &pipeline_info.short_commit_sha,
        scope,
        stage,
        candidate: args.candidate.as_deref(),
        target: target.as_ref(),
        tag_names: &tag_names,
    })?;

    println!("UPCOMING_VERSION={}", versions.upcoming_version);
    println!("LAST_VERSION={}", versions.last_version);

    Ok(())
}

/// Read patterns only for missing options, preserving stage-before-scope errors.
/// Branch rules receive resolved settings and do not read the environment.
fn resolve_policy(branch: &str, args: &VersionCommandArgs) -> Result<(Scope, Stage)> {
    let has_candidate = args.candidate.is_some();
    let stage = match args.stage {
        Some(stage) => stage,
        // Preserve the existing explicit release scope on any named branch.
        None if args.scope == Some(Scope::Release) => Stage::Stable,
        None => StagePatterns {
            dev: config::env_var_or("DEV", DEV_PATTERN)?,
            rc: config::env_var_or("RC", RC_PATTERN)?,
            stable: config::env_var_or("STABLE", STABLE_PATTERN)?,
        }
        .resolve(branch)?,
    };
    branch_rules::validate_stage(stage, has_candidate)?;

    let scope = match args.scope {
        Some(scope) => scope,
        None if has_candidate => Scope::Release,
        None => ScopePatterns {
            major: config::env_var_or("MAJOR", MAJOR_PATTERN)?,
            minor: config::env_var_or("MINOR", MINOR_PATTERN)?,
            patch: config::env_var_or("PATCH", PATCH_PATTERN)?,
            release: config::env_var_or("RELEASE", STABLE_PATTERN)?,
        }
        .resolve(branch)?,
    };
    branch_rules::validate_scope(scope, stage, has_candidate)?;
    Ok((scope, stage))
}
