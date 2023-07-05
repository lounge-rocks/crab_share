use std::{
    env,
    error::Error,
    fmt::{Display, Formatter},
};

use s3::creds::error::CredentialsError;

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
    Missing(String),
    Credentials(CredentialsError),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO Error: {}", e),
            ConfigError::Parse(e) => write!(f, "{}", e),
            ConfigError::Missing(e) => write!(f, "Missing config option: {}", e),
            ConfigError::Credentials(e) => write!(f, "Credentials Error: {}", e),
        }
    }
}

impl Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<env::VarError> for ConfigError {
    fn from(e: env::VarError) -> Self {
        ConfigError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

impl From<CredentialsError> for ConfigError {
    fn from(e: CredentialsError) -> Self {
        ConfigError::Credentials(e)
    }
}
