use bytesize::ByteSize;
use clap::Parser;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

use s3::creds::{error::CredentialsError, Credentials};
use s3::{Bucket, Region};

mod zip;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// How long the link should be valid for (default: 7d)
    #[arg(short, long, default_value = "7d")]
    expires: String,

    /// Path to upload. If it is a directory, it will be zipped.
    #[arg()]
    path: PathBuf,
}

#[derive(Deserialize, Debug, Clone)]
struct JSONCredentials {
    url: String,
    #[serde(rename = "accessKey")]
    access_key: String,
    #[serde(rename = "secretKey")]
    secret_key: String,
}

impl TryInto<Credentials> for JSONCredentials {
    type Error = CredentialsError;

    fn try_into(self) -> Result<Credentials, Self::Error> {
        Credentials::new(
            Some(&self.access_key),
            Some(&self.secret_key),
            None,
            None,
            None,
        )
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.path.exists() {
        println!("path does not exist");
        exit(1);
    }

    let json_credentials = match get_creds_from_env() {
        Ok(c) => c,
        Err(_) => {
            // read ~/.aws/credentials.json
            let path = Path::new(&env::var("HOME").expect("HOME env var not set"))
                .join(".aws")
                .join("credentials.json");
            let cred_file = match fs::read_to_string(path) {
                Ok(f) => f,
                Err(e) => {
                    println!("error reading credentials file: {}", e);
                    exit(1);
                }
            };
            match serde_json::from_str(&cred_file) {
                Ok(c) => c,
                Err(e) => {
                    println!("error parsing credentials file: {}", e);
                    exit(1);
                }
            }
        }
    };

    let credentials: Credentials = json_credentials.clone().try_into()?;

    // connect to s3
    let region = Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: json_credentials.url,
    };

    let bucket = Bucket::new("sharepy", region, credentials)?.with_path_style();

    // 1. Upload a file to the bucket.
    // <uuid>/filename

    let path = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| panic!("Path could not be canonicalized: {:?}", args.path));
    let mut file_name = path
        .file_name()
        .expect("A canonicalized path should have a file name")
        .to_string_lossy();

    // 1.0. Check if file is a directory
    let content = match args.path.is_dir() {
        true => {
            println!("zipping directory...");
            let src_dir = args.path.canonicalize()?.to_string_lossy().to_string();
            file_name = (file_name.to_string() + ".zip").into();
            zip::zip_folder(&src_dir)?
        }
        false => fs::read(&args.path)?,
    };

    // 1.1. Read file
    // 1.2. Create path
    let path = uuid::Uuid::new_v4().to_string() + "/" + file_name.as_ref();
    // 1.3. Upload file to bucket
    println!(
        "uploading file with size {} bytes to {} ...",
        ByteSize(content.len() as u64),
        path
    );
    let reponse = bucket.put_object_blocking(&path, &content)?;
    if reponse.status_code() != 200 {
        println!("error uploading file: {:?}", reponse);
        exit(1);
    }

    // 2. Get the url of the file
    // -> presigned url

    // 2.1. Create presigned url
    let expiry_secs = match get_time_from_str(&args.expires) {
        Some(s) => s,
        None => {
            println!("invalid expiry time");
            exit(1);
        }
    };
    let url = match bucket.presign_get(&path, expiry_secs, None) {
        Ok(u) => u,
        Err(e) => {
            println!("error creating presigned url: {}", e);
            exit(1);
        }
    };

    // 2.2. Print url
    println!("\n{}", url);

    Ok(())
}

fn get_creds_from_env() -> Result<JSONCredentials, Box<dyn std::error::Error>> {
    let url = env::var("S3_URL")?;
    let access_key = env::var("S3_ACCESS_KEY")?;
    let secret_key = env::var("S3_SECRET_KEY")?;
    Ok(JSONCredentials {
        url,
        access_key,
        secret_key,
    })
}
