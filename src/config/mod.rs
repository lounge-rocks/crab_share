mod error;
use self::error::*;

mod json;
use self::json::*;

mod args;
use self::args::*;

mod env;
use self::env::*;

use std::path::PathBuf;

use clap::Parser;
use rusty_s3::Credentials;
use serde::Deserialize;
use zip::CompressionMethod;

#[derive(Debug, Deserialize, Clone)]
pub enum CompressionMthd {
    #[serde(rename = "deflate")]
    Deflate,
    #[serde(rename = "bzip2")]
    Bzip2,
    #[serde(rename = "stored")]
    Stored,
    #[serde(rename = "zstd")]
    Zstd,
}

impl From<CompressionMthd> for zip::CompressionMethod {
    fn from(mthd: CompressionMthd) -> Self {
        match mthd {
            CompressionMthd::Deflate => zip::CompressionMethod::Deflated,
            CompressionMthd::Bzip2 => zip::CompressionMethod::Bzip2,
            CompressionMthd::Stored => zip::CompressionMethod::Stored,
            CompressionMthd::Zstd => zip::CompressionMethod::Zstd,
        }
    }
}

impl From<String> for CompressionMthd {
    fn from(mthd: String) -> Self {
        match mthd.as_str() {
            "deflate" => CompressionMthd::Deflate,
            "bzip2" => CompressionMthd::Bzip2,
            "stored" | "store" => CompressionMthd::Stored,
            "zstd" => CompressionMthd::Zstd,
            _ => panic!("Invalid compression method: {}", mthd),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// How long the link should be valid for in seconds (default: 7d)
    pub expires: u32,
    /// Which bucket to upload to
    pub bucket: String,
    /// What URL to use
    pub url: String,
    /// Path to upload. If it is a directory, it will be zipped.
    pub path: PathBuf,
    /// Aws credentials
    pub credentials: Credentials,
    /// Aws region (default: eu-central-1)
    pub region: String,
    /// How to compress the zip file (default: deflate)
    pub compression: CompressionMethod,
    /// Whether to zip a single file
    pub zip_single_file: bool,
    /// Whether to purge expired files before uploading
    pub purge: bool,
}

/// Partial config: All possible config options, all optional. To be merged with other configs.
#[derive(Debug, Default)]
struct PartialConfig {
    /// How long the link should be valid for in seconds (default: 7d)
    expires: Option<String>,
    /// Which bucket to upload to
    bucket: Option<String>,
    /// What URL to use
    url: Option<String>,
    /// Path to upload. If it is a directory, it will be zipped.
    path: Option<PathBuf>,
    /// The region to use (default: eu-central-1)
    region: Option<String>,
    /// Aws credentials
    credentials: Option<Credentials>,
    /// How to compress the zip file (default: deflate)
    compression: Option<CompressionMethod>,
    /// Whether to zip a single file
    zip_single_file: Option<bool>,
    /// Whether to purge expired files before uploading
    purge: Option<bool>,
}

impl PartialConfig {
    fn merge(self, other: PartialConfig) -> PartialConfig {
        PartialConfig {
            expires: self.expires.or(other.expires),
            bucket: self.bucket.or(other.bucket),
            url: self.url.or(other.url),
            path: self.path.or(other.path),
            region: self.region.or(other.region),
            credentials: self.credentials.or(other.credentials),
            compression: self.compression.or(other.compression),
            zip_single_file: self.zip_single_file.or(other.zip_single_file),
            purge: self.purge.or(other.purge),
        }
    }

    fn static_default() -> PartialConfig {
        PartialConfig {
            expires: Some("7d".to_string()),
            bucket: None,
            url: None,
            path: None,
            region: Some("eu-central-1".to_string()),
            credentials: None,
            compression: Some(CompressionMethod::Zstd),
            zip_single_file: Some(false),
            purge: Some(false),
        }
    }
}

impl Config {
    pub fn parse() -> Result<Self, ConfigError> {
        let args = Args::parse();
        let args_config = PartialConfig::from(args);

        let env_config = EnvConf::get_from_env().into();
        let partial_config = args_config.merge(env_config);

        // try to read ~/.aws/crab_share.json
        let json_config = JSONConfig::get_from_file();
        let partial_config = if let Ok(json_config) = json_config {
            partial_config.merge(PartialConfig::from(json_config))
        } else {
            println!(
                "Warn: Could not read ~/.aws/crab_share.json: {}",
                json_config.unwrap_err()
            );
            partial_config
        };

        let partial_config_creds: Result<PartialConfig, ConfigError> =
            JSONCredentials::get_from_file().map(|c| c.into());
        let partial_config = if let Ok(partial_config_creds) = partial_config_creds {
            partial_config.merge(partial_config_creds)
        } else {
            println!(
                "Warn: Could not read ~/.aws/credentials.json: {}",
                partial_config_creds.unwrap_err()
            );
            partial_config
        };

        // fill the rest with the static defaults
        let partial_config = partial_config.merge(PartialConfig::static_default());

        if let Some(path) = &partial_config.path {
            if !path.exists() {
                return Err(ConfigError::Parse(format!(
                    "Path {} does not exist",
                    path.display()
                )));
            }
        } else {
            return Err(ConfigError::Parse("No path given".to_string()));
        }
        Ok(Config {
            expires: partial_config
                .expires
                .as_deref()
                .map(get_time_from_str)
                .expect("expires should always be set by static default")
                .ok_or_else(move || {
                    ConfigError::Parse(format!(
                        "Could not parse expires: \"{}\"",
                        partial_config.expires.unwrap()
                    ))
                })?,
            bucket: partial_config
                .bucket
                .ok_or_else(|| ConfigError::Missing("bucket".to_string()))?,
            region: partial_config
                .region
                .expect("Region should always be set by static default"),
            url: partial_config
                .url
                // make into error
                .ok_or(ConfigError::Missing("url".to_string()))?,
            path: partial_config
                .path
                .ok_or_else(|| ConfigError::Missing("path".to_string()))?
                .canonicalize()
                .map_err(|e| ConfigError::Parse(format!("Could not canonicalize path: {}", e)))?,
            credentials: partial_config
                .credentials
                .ok_or_else(|| ConfigError::Missing("credentials".to_string()))?,
            compression: partial_config
                .compression
                .expect("Compression should always be set by static default"),
            zip_single_file: partial_config
                .zip_single_file
                .expect("zip_single_file should always be set by static default"),
            purge: partial_config
                .purge
                .expect("purge should always be set by static default"),
        })
    }
}

/// calculate the time from a string
/// for example: 7d -> 7 days (in seconds)
fn get_time_from_str(input: &str) -> Option<u32> {
    let (time, denom) = input.split_at(input.len() - 1);
    match denom.chars().next()? {
        'd' => Some(time.parse::<u32>().ok()? * 24 * 60 * 60),
        'h' => Some(time.parse::<u32>().ok()? * 60 * 60),
        'm' => Some(time.parse::<u32>().ok()? * 60),
        's' => Some(time.parse::<u32>().ok()?),
        _ => Some(input.parse::<u32>().ok()?),
    }
}
