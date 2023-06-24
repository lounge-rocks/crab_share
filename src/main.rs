use clap::Parser;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

use s3::creds::{error::CredentialsError, Credentials};
use s3::{Bucket, Region};

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// Time to wait before greeting
    #[arg(short, long, default_value = "7d")]
    expires: String,

    /// Number of times to greet
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
fn get_time_from_str(input: &str) -> u32 {
    let (time, denom) = input.split_at(input.len() - 1);
    match denom.chars().next().unwrap() {
        'd' => time.parse::<u32>().unwrap() * 24 * 60 * 60,
        'h' => time.parse::<u32>().unwrap() * 60 * 60,
        'm' => time.parse::<u32>().unwrap() * 60,
        's' => time.parse::<u32>().unwrap(),
        _ => input.parse::<u32>().unwrap(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.path.exists() {
        println!("path does not exist");
        exit(1);
    }

    // read ~/.aws/credentials.json
    let path = Path::new(&env::var("HOME").unwrap())
        .join(".aws")
        .join("credentials.json");
    let cred_file = match fs::read_to_string(path) {
        Ok(f) => f,
        Err(e) => {
            println!("error reading credentials file: {}", e);
            exit(1);
        }
    };
    let json_credentials: JSONCredentials = match serde_json::from_str(&cred_file) {
        Ok(c) => c,
        Err(e) => {
            println!("error parsing credentials file: {}", e);
            exit(1);
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

    // 1.1. Read file
    let file = fs::read(&args.path)?;
    // 1.2. Create path
    let path =
        uuid::Uuid::new_v4().to_string() + "/" + args.path.file_name().unwrap().to_str().unwrap();
    // 1.3. Upload file to bucket
    let reponse = bucket.put_object_blocking(&path, &file)?;
    if reponse.status_code() != 200 {
        println!("error uploading file: {:?}", reponse);
        exit(1);
    }

    // 2. Get the url of the file
    // -> presigned url

    // 2.1. Create presigned url
    let url = bucket.presign_get(&path, get_time_from_str(&args.expires), None)?;

    // 2.2. Print url
    println!("{}", url);

    Ok(())
}
