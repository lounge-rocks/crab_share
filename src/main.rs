use bytesize::ByteSize;
use config::Config;
use std::{fs, process::exit, time::Duration};

use reqwest::header::ETAG;
use reqwest::Client;
use rusty_s3::actions::{
    CompleteMultipartUpload, CreateMultipartUpload, GetObject, PutObject, S3Action, UploadPart,
};
use rusty_s3::{Bucket, UrlStyle};

mod config;
mod zip;

const ONE_HOUR: Duration = Duration::from_secs(3600);

#[tokio::main]
async fn main() {
    let config = match config::Config::parse() {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };

    // connect to s3
    let url = match config.url.parse() {
        Ok(u) => u,
        Err(e) => {
            println!("error parsing url: {}", e);
            exit(1);
        }
    };
    let client = Client::new();
    let bucket = match Bucket::new(
        url,
        UrlStyle::Path,
        config.bucket.clone(),
        config.region.clone(),
    ) {
        Ok(b) => b,
        Err(e) => {
            println!("error creating bucket: {}", e);
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
    if content.len() > 100 * 1024 * 1024 {
        println!("file too large (> 5MB), uploading with multipart upload");
        let now = std::time::Instant::now();
        let action = CreateMultipartUpload::new(&bucket, Some(&config.credentials), &path);

        let url = action.sign(ONE_HOUR);

        let resp = client.post(url);
        let resp = match resp.send().await {
            Ok(r) => r,
            Err(e) => {
                println!("error creating multipart upload: {}", e);
                exit(1);
            }
        };
        let resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                println!("error creating multipart upload: {}", e);
                exit(1);
            }
        };

        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                println!("error creating multipart upload: {}", e);
                exit(1);
            }
        };

        let upload = match CreateMultipartUpload::parse_response(&body) {
            Ok(u) => u,
            Err(e) => {
                println!("error creating multipart upload: {}", e);
                exit(1);
            }
        };
        println!(
            "initiated multipart upload in {}ms",
            now.elapsed().as_millis()
        );
        let now = std::time::Instant::now();
        let mut parts = Vec::new();
        for (i, chunk) in content.chunks(100 * 1024 * 1024).enumerate() {
            let etag = upload_part(
                &bucket,
                &config,
                &path,
                i as u16,
                upload.upload_id(),
                &client,
                chunk,
            )
            .await;
            parts.push(etag);
        }
        println!(
            "uploaded {} chunks in {}ms",
            parts.len(),
            now.elapsed().as_millis()
        );
        println!(
            "That is a speed of {}/s",
            ByteSize(content.len() as u64 / now.elapsed().as_secs())
        );

        let action = CompleteMultipartUpload::new(
            &bucket,
            Some(&config.credentials),
            &path,
            upload.upload_id(),
            parts.iter().map(|p| p.as_str()),
        );
        let url = action.sign(ONE_HOUR);

        let resp = match client.post(url).body(action.body()).send().await {
            Ok(r) => r,
            Err(e) => {
                println!("error completing multipart upload: {}", e);
                exit(1);
            }
        };
        match resp.error_for_status() {
            Ok(_) => {}
            Err(e) => {
                println!("error completing multipart upload: {}", e);
                exit(1);
            }
        }
    } else {
        println!("uploading file with single upload");
        let now = std::time::Instant::now();
        let action = PutObject::new(&bucket, Some(&config.credentials), &path);
        let url = action.sign(ONE_HOUR);
        let content_len = content.len();
        let resp = match client.put(url).body(content).send().await {
            Ok(r) => r,
            Err(e) => {
                println!("error uploading file: {}", e);
                exit(1);
            }
        };
        match resp.error_for_status() {
            Ok(_) => {}
            Err(e) => {
                println!("error uploading file: {}", e);
                exit(1);
            }
        }
        println!("uploaded file in {}ms", now.elapsed().as_millis());
        println!(
            "That is a speed of {}/s",
            ByteSize(content_len as u64 / now.elapsed().as_secs())
        );
    }

    // 2. Get the url of the file
    // -> presigned url

    // 2.1. Create presigned url
    let mut action = GetObject::new(&bucket, Some(&config.credentials), &path);
    action
        .query_mut()
        .insert("response-cache-control", "no-cache, no-store");
    let url = action.sign(ONE_HOUR);

    // 2.2. Print url
    println!("\n{}", url);
}

async fn upload_part(
    bucket: &Bucket,
    config: &Config,
    path: &str,
    i: u16,
    upload_id: &str,
    client: &Client,
    chunk: &[u8],
) -> String {
    // println!(
    //     "uploading chunk {i}/{n}",
    //     i = i,
    //     n = content.len() / 100 / 1024 / 1024
    // );
    let action = UploadPart::new(bucket, Some(&config.credentials), path, i + 1, upload_id);
    let url = action.sign(ONE_HOUR);
    let resp = match client.put(url).body(chunk.to_vec()).send().await {
        Ok(r) => r,
        Err(e) => {
            println!("error uploading chunk: {}", e);
            exit(1);
        }
    };
    let resp = match resp.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            println!("error uploading chunk: {}", e);
            exit(1);
        }
    };
    let etag = match resp.headers().get(ETAG) {
        Some(e) => e,
        None => {
            println!("error uploading chunk: no etag in response");
            exit(1);
        }
    };
    let etag = match etag.to_str() {
        Ok(e) => e,
        Err(e) => {
            println!("error uploading chunk: {}", e);
            exit(1);
        }
    };
    etag.to_string()
}
