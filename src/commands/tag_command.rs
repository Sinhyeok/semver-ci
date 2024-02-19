use crate::{git_service, pipelines};
use clap::Args;

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
    let pipeline_info = pipelines::pipeline_info();

    let mut tag_name = args.tag_name.as_str();
    if args.strip_prefix_v {
        tag_name = tag_name.strip_prefix('v').unwrap()
    };

    let tag_message = args.tag_message.as_str();

    git_service::tag_and_push(&pipeline_info, tag_name, tag_message)
        .unwrap_or_else(|e| panic!("{}", e));
}
