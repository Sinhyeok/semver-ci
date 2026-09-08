use crate::default_error::Result;
use crate::pipelines;
use crate::release::Release;
use clap::Args;

#[derive(Args)]
pub(crate) struct ReleaseCommandArgs {
    /// Release name
    #[arg()]
    name: String,

    /// Release description
    #[arg(long, env, default_value = "")]
    description: String,

    #[arg(long, env)]
    tag_name: Option<String>,

    /// Specify tag_message to create an annotated tag
    #[arg(long, env, default_value = "")]
    tag_message: String,

    /// Automatically generate the body for this release. If description is specified, the description will be pre-pended to the automatically generated notes.
    #[arg(short, long, env, action)]
    generate_release_notes: bool,

    /// (Only for GitLab CI) tag from previous releases to compare when automatically generating release notes
    #[arg(short, long, env, default_value = "")]
    previous_tag: String,

    /// Strip one leading lowercase "v" from the release name and tag name.
    /// ex) v0.1.0 => 0.1.0
    #[arg(short, long, env, action)]
    strip_prefix_v: bool,
}

pub(crate) fn run(args: ReleaseCommandArgs) -> Result<()> {
    let mut name = args.name.as_str();
    let mut tag_name = args.tag_name.as_deref().unwrap_or(name);
    if args.strip_prefix_v {
        name = name.strip_prefix('v').unwrap_or(name);
        tag_name = tag_name.strip_prefix('v').unwrap_or(tag_name);
    }

    let release = Release {
        name: name.to_owned(),
        description: args.description,
        tag_name: tag_name.to_owned(),
        tag_message: args.tag_message,
        generate_release_notes: args.generate_release_notes,
        previous_tag: args.previous_tag,
    };

    let pipeline = pipelines::current_pipeline()?;
    let parsed = pipeline.create_release(&release)?;

    println!("{:#?}", parsed);

    Ok(())
}
