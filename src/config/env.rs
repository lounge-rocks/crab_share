use super::PartialConfig;

use s3::creds::{error::CredentialsError, Credentials};
use std::{env, path::PathBuf};

#[derive(Debug, Clone, Default)]
pub(crate) struct EnvConf {
    url: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    session_token: Option<String>,
    security_token: Option<String>,
    profile: Option<String>,

    /// How long the link should be valid for in seconds (default: 7d)
    expires: Option<String>,
    /// Which bucket to upload to
    bucket: Option<String>,
    /// What URL to use
    /// Path to upload. If it is a directory, it will be zipped.
    path: Option<PathBuf>,
    /// The region to use (default: eu-central-1)
    region: Option<String>,
}

impl TryInto<Credentials> for EnvConf {
    type Error = CredentialsError;

    fn try_into(self) -> Result<Credentials, Self::Error> {
        Credentials::new(
            self.access_key.as_deref(),
            self.secret_key.as_deref(),
            self.session_token.as_deref(),
            self.security_token.as_deref(),
            self.profile.as_deref(),
        )
    }
}

impl From<EnvConf> for PartialConfig {
    fn from(json_credentials: EnvConf) -> Self {
        // prevent panic from library
        let credentials = if json_credentials.access_key.is_some() {
            json_credentials.clone().try_into().ok()
        } else {
            None
        };

        PartialConfig {
            credentials,
            url: json_credentials.url,
            expires: json_credentials.expires,
            bucket: json_credentials.bucket,
            path: json_credentials.path,
            region: json_credentials.region,
        }
    }
}

impl EnvConf {
    pub(crate) fn get_from_env() -> Self {
        let url = env::var("S3_URL").ok();
        let access_key = env::var("S3_ACCESS_KEY").ok();
        let secret_key = env::var("S3_SECRET_KEY").ok();
        let session_token = env::var("S3_SESSION_TOKEN").ok();
        let security_token = env::var("S3_SECURITY_TOKEN").ok();
        let profile = env::var("S3_PROFILE").ok();

        let expires = env::var("S3_EXPIRES").ok();
        let bucket = env::var("S3_BUCKET").ok();
        let path = env::var("S3_PATH").ok().map(PathBuf::from);
        let region = env::var("S3_REGION").ok();
        EnvConf {
            url,
            access_key,
            secret_key,
            session_token,
            security_token,
            profile,
            expires,
            bucket,
            path,
            region,
        }
    }
}
