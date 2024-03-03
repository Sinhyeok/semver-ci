use crate::release::Release;
use clap::Args;

#[derive(Args)]
pub(crate) struct ReleaseCommandArgs {
    /// Release name
    #[arg()]
    name: String,
    #[arg(long, env, default_value = "")]
    description: String,
    #[arg(long, env)]
    tag_name: Option<String>,
    /// Specify tag_message to create an annotated tag
    #[arg(long, env, default_value = "")]
    tag_message: String,
    /// (Only for Github Actions) Automatically generate the body for this release. If body is specified, the body will be pre-pended to the automatically generated notes.
    #[arg(short, long, env, action)]
    generate_release_notes: bool,
    /// Strip prefix "v" from release name and tag name.
    /// ex) v0.1.0 => 0.1.0
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
        generate_release_notes: args.generate_release_notes,
    };

    let parsed = release.create();
    println!("{:#?}", parsed);
}
