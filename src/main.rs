use bytesize::ByteSize;
use std::{fs, process::exit};

use s3::{Bucket, Region};

mod config;
mod zip;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::parse()?;

    // connect to s3
    let region = Region::Custom {
        region: config.bucket.clone(),
        endpoint: config.url,
    };

    let bucket = Bucket::new(&config.bucket, region, config.credentials)?.with_path_style();

    // 1. Upload a file to the bucket.
    // <uuid>/filename

    let path = config
        .path
        .canonicalize()
        .unwrap_or_else(|_| panic!("Path could not be canonicalized: {:?}", config.path));
    let mut file_name = path
        .file_name()
        .expect("A canonicalized path should have a file name")
        .to_string_lossy();

    // 1.0. Check if file is a directory
    let content = match config.path.is_dir() {
        true => {
            println!("zipping directory...");
            let src_dir = config.path.canonicalize()?.to_string_lossy().to_string();
            file_name = (file_name.to_string() + ".zip").into();
            zip::zip_folder(&src_dir)?
        }
        false => fs::read(&config.path)?,
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
    let url = match bucket.presign_get(&path, config.expires, None) {
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
