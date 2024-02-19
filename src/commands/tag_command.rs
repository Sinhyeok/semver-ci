use crate::{git_service, pipelines};
use clap::Args;

#[derive(Args)]
pub(crate) struct TagCommandArgs {
    #[arg()]
    tag_name: String,
    #[arg(short, long, env, action)]
    strip_prefix_v: bool,
}

pub(crate) fn run(args: TagCommandArgs) {
    let pipeline_info = pipelines::pipeline_info();

    let mut tag_name = args.tag_name.as_str();
    if args.strip_prefix_v {
        tag_name = tag_name.strip_prefix('v').unwrap()
    };

    git_service::tag_and_push(&pipeline_info, tag_name, "").unwrap_or_else(|e| panic!("{}", e));
}
