use crate::default_error::DefaultError;
use crate::pipelines::PipelineInfo;
use crate::{config, git_service, pipelines};
use clap::Args;
use git2::Repository;
use std::error::Error;

#[derive(Args)]
pub(crate) struct TagCommandArgs {
    #[arg()]
    tag_name: String,
    #[arg(long, env, default_value = "")]
    tag_message: String,
    #[arg(short, long, env, action)]
    strip_prefix_v: bool,
}

pub(crate) fn run(args: TagCommandArgs) -> Result<(), Box<dyn Error>> {
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipeline.info();

    let mut tag_name = args.tag_name.as_str();
    if args.strip_prefix_v {
        if let Some(stripped) = tag_name.strip_prefix('v') {
            tag_name = stripped
        }
    };

    let tag_message = args.tag_message.as_str();

    tag_and_push(&pipeline_info, tag_name, tag_message)
}

fn tag_and_push(
    pipeline_info: &PipelineInfo,
    tag_name: &str,
    tag_message: &str,
) -> Result<(), Box<dyn Error>> {
    let repo = Repository::open(config::clone_target_path())?;

    git_service::tag(
        &repo,
        tag_name,
        tag_message,
        &pipeline_info.git_username,
        &pipeline_info.git_email,
    )?;

    Ok(git_service::push_tag(
        &repo,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
        tag_name,
    )
    .map_err(|e| {
        Box::new(DefaultError {
            message: "Failed to push tag".to_string(),
            source: Some(Box::new(e)),
        })
    })?)
}
