use bytesize::ByteSize;
use std::{fs, process::exit};

use s3::{Bucket, Region};

mod config;
mod zip;

fn main() {
    let config = match config::Config::parse() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };

    // connect to s3
    let region = Region::Custom {
        region: config.bucket.clone(),
        endpoint: config.url,
    };

    let bucket = match Bucket::new(&config.bucket, region, config.credentials) {
        Ok(b) => b.with_path_style(),
        Err(e) => {
            println!("error connecting to s3: {}", e);
            exit(1);
        }
    };

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
            let src_dir = config.path.to_string_lossy().to_string();
            file_name = (file_name.to_string() + ".zip").into();
            match zip::zip_folder(&src_dir) {
                Ok(c) => c,
                Err(e) => {
                    println!("error zipping directory: {}", e);
                    exit(1);
                }
            }
        }
        false => match fs::read(&config.path) {
            Ok(c) => c,
            Err(e) => {
                println!("error reading file: {}", e);
                exit(1);
            }
        },
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
    let reponse = match bucket.put_object_blocking(&path, &content) {
        Ok(r) => r,
        Err(e) => {
            println!("error uploading file: {}", e);
            exit(1);
        }
    };
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
}
