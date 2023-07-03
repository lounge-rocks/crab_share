use std::path::PathBuf;

use clap::Parser;

use super::PartialConfig;

#[derive(Parser, Debug)]
#[command(author, version)]
pub(crate) struct Args {
    /// How long the link should be valid for (default: 7d)
    #[arg(short, long)]
    expires: Option<String>,

    /// Which bucket to upload to
    #[arg(short, long)]
    bucket: Option<String>,

    /// What URL to use
    #[arg(short, long)]
    url: Option<String>,

    /// Path to upload. If it is a directory, it will be zipped.
    #[arg()]
    path: PathBuf,
}

impl From<Args> for PartialConfig {
    fn from(args: Args) -> Self {
        PartialConfig {
            expires: args.expires,
            bucket: args.bucket,
            url: args.url,
            path: Some(args.path),
            region: None,
            credentials: None,
        }
    }
}
