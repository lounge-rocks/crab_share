use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use rusty_s3::Credentials;

use super::{CompressionMthd, PartialConfig};

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

    /// Which region to use (default: eu-central-1)
    #[arg(short, long)]
    region: Option<String>,

    /// S3 access key
    #[arg(short, long)]
    access_key: Option<String>,

    /// S3 secret key
    #[arg(short, long)]
    secret_key: Option<String>,

    /// How to compress into zip file (default: deflate)
    #[arg(short, long)]
    compression: Option<CompressionMthd>,

    /// Whether to zip a single file
    #[arg(short, long)]
    zip_single_file: bool,

    /// Whether to purge expired files before uploading
    #[arg(short, long)]
    purge: bool,

    /// Generate shell completion script for the specified shell
    #[arg(long, value_name = "SHELL")]
    generate_completion: Option<Shell>,

    /// Path to upload. If it is a directory, it will be zipped.
    #[arg()]
    path: Option<PathBuf>,
}

impl Args {
    pub fn generate_completion_if_requested(&self) {
        if let Some(shell) = self.generate_completion {
            let mut cmd = Args::command();
            let bin_name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
            std::process::exit(0);
        }
    }
}

impl From<Args> for PartialConfig {
    fn from(args: Args) -> Self {
        let credentials = match (args.access_key, args.secret_key) {
            (Some(access_key), Some(secret_key)) => Some(Credentials::new(access_key, secret_key)),
            _ => None,
        };
        let zip_single_file = if args.zip_single_file {
            Some(true)
        } else {
            None
        };
        let purge = if args.purge { Some(true) } else { None };
        PartialConfig {
            expires: args.expires,
            bucket: args.bucket,
            url: args.url,
            path: args.path,
            region: args.region,
            credentials,
            compression: args.compression.map(|mthd| mthd.into()),
            zip_single_file,
            purge,
        }
    }
}
