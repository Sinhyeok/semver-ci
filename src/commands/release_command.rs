use crate::release::Release;
use clap::Args;

#[derive(Args)]
pub(crate) struct ReleaseCommandArgs {
    #[arg()]
    name: String,
    #[arg(long, env, default_value = "")]
    description: String,
    #[arg(long, env)]
    tag_name: Option<String>,
    #[arg(long, env, default_value = "")]
    tag_message: String,
    #[arg(short, long, env, action)]
    strip_prefix_v: bool,
}

pub(crate) fn run(args: ReleaseCommandArgs) {
    let tag_name = args.tag_name.unwrap_or(args.name.clone());
    let release = Release {
        name: args.name,
        description: args.description,
        tag_name,
        tag_message: args.tag_message,
    };
    release.create();
}
