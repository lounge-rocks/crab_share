use std::{env, fs, path::Path};

use rusty_s3::Credentials;
use serde::Deserialize;

use super::{error::ConfigError, CompressionMthd, PartialConfig};

#[derive(Deserialize, Debug, Clone, Default)]
pub(crate) struct JSONCredentials {
    url: Option<String>,
    #[serde(rename = "accessKey")]
    access_key: Option<String>,
    #[serde(rename = "secretKey")]
    secret_key: Option<String>,
}

impl TryInto<Credentials> for JSONCredentials {
    type Error = ();

    fn try_into(self) -> Result<Credentials, Self::Error> {
        match (self.access_key, self.secret_key) {
            (Some(access_key), Some(secret_key)) => Ok(Credentials::new(access_key, secret_key)),
            _ => Err(()),
        }
    }
}

impl From<JSONCredentials> for PartialConfig {
    fn from(json_credentials: JSONCredentials) -> Self {
        // prevent panic from library
        let credentials = if json_credentials.access_key.is_some() {
            json_credentials.clone().try_into().ok()
        } else {
            None
        };

        PartialConfig {
            url: json_credentials.url,
            credentials,
            ..PartialConfig::default()
        }
    }
}

impl JSONCredentials {
    pub(crate) fn get_from_file() -> Result<Self, ConfigError> {
        let path = Path::new(&env::var("HOME").expect("HOME env var not set"))
            .join(".aws")
            .join("credentials.json");
        let cred_file = fs::read_to_string(path)?;
        serde_json::from_str(&cred_file)
            .map_err(|e| ConfigError::Parse(format!("error parsing credentials file: {e}",)))
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub(crate) struct JSONConfig {
    bucket: Option<String>,
    region: Option<String>,
    url: Option<String>,
    expires: Option<String>,
    compression: Option<CompressionMthd>,
    #[serde(rename = "zipSingleFile")]
    zip_single_file: Option<bool>,
}

impl From<JSONConfig> for PartialConfig {
    fn from(json_config: JSONConfig) -> Self {
        PartialConfig {
            bucket: json_config.bucket,
            region: json_config.region,
            url: json_config.url,
            path: None,
            credentials: None,
            expires: json_config.expires,
            compression: json_config.compression.map(|c| c.into()),
            zip_single_file: json_config.zip_single_file,
        }
    }
}

impl JSONConfig {
    pub(crate) fn get_from_file() -> Result<Self, ConfigError> {
        let path = Path::new(&env::var("HOME").expect("HOME env var not set"))
            .join(".aws")
            .join("crab_share.json");
        let config_file = fs::read_to_string(&path)?;
        serde_json::from_str(&config_file).map_err(|e| {
            ConfigError::Parse(format!("error parsing config file {}: {e}", path.display(),))
        })
    }
}
