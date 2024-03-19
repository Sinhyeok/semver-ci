use crate::pipelines::PipelineInfo;
use crate::{git_service, pipelines};
use clap::Args;
use git2::{Error, Repository};

#[derive(Args)]
pub(crate) struct TagCommandArgs {
    #[arg()]
    tag_name: String,
    #[arg(long, env, default_value = "")]
    tag_message: String,
    #[arg(short, long, env, action)]
    strip_prefix_v: bool,
}

pub(crate) fn run(args: TagCommandArgs) {
    let pipeline = pipelines::current_pipeline();
    pipeline.init();
    let pipeline_info = pipeline.info();

    let mut tag_name = args.tag_name.as_str();
    if args.strip_prefix_v {
        tag_name = tag_name.strip_prefix('v').unwrap()
    };

    let tag_message = args.tag_message.as_str();

    tag_and_push(&pipeline_info, tag_name, tag_message).unwrap_or_else(|e| panic!("{}", e));
}

fn tag_and_push(
    pipeline_info: &PipelineInfo,
    tag_name: &str,
    tag_message: &str,
) -> Result<(), Error> {
    let repo = Repository::open(&pipeline_info.target_path)?;

    git_service::tag(
        &repo,
        tag_name,
        tag_message,
        &pipeline_info.git_username,
        &pipeline_info.git_email,
    )?;

    git_service::push_tag(
        &repo,
        &pipeline_info.git_username,
        &pipeline_info.git_token,
        tag_name,
    )
}
