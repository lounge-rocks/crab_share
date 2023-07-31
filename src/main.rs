use bytesize::ByteSize;
use indicatif::{ProgressBar, ProgressStyle};
use std::{fs, process::exit, time::Duration};

use reqwest::Client;
use rusty_s3::actions::{
    CompleteMultipartUpload, CreateMultipartUpload, GetObject, PutObject, S3Action,
};
use rusty_s3::{Bucket, UrlStyle};

use crate::upload_actor::spawn_upload_actor;

mod config;
mod purge;
mod upload_actor;
mod zip;

const ONE_HOUR: Duration = Duration::from_secs(3600);

#[tokio::main]
async fn main() {
    let config = match config::Config::parse() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };
    let chunk_size = 16 * 1024 * 1024;
    let num_threads = 8;

    // connect to s3
    let url = match config.url.parse() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error parsing url: {}", e);
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
            eprintln!("error creating bucket: {}", e);
            exit(1);
        }
    };

    if config.purge {
        purge::purge(&config, &bucket).await;
    }

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
    let content = match (config.path.is_dir(), config.zip_single_file) {
        (true, _) => {
            println!("zipping directory...");
            let src_dir = config.path.to_string_lossy().to_string();
            file_name = (file_name.to_string() + ".zip").into();
            match zip::zip_folder(&src_dir, config.compression) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error zipping directory: {}", e);
                    exit(1);
                }
            }
        }
        (_, true) => {
            println!("zipping file...");
            let src_dir = config.path.to_string_lossy().to_string();
            file_name = (file_name.to_string() + ".zip").into();
            match zip::zip_file(&src_dir, config.compression) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error zipping file: {}", e);
                    exit(1);
                }
            }
        }
        _ => match fs::read(&config.path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error reading file: {}", e);
                exit(1);
            }
        },
    };

    // 1.1. Read file
    // 1.2. Create path
    let ulid = {
        let expiry = std::time::SystemTime::now() + Duration::from_secs(config.expires.into());
        ulid::Ulid::from_datetime(expiry).to_string()
    };
    let path = ulid + "/" + file_name.as_ref();
    // 1.3. Upload file to bucket
    println!(
        "uploading file with size {} bytes to {}/{}/{} ...",
        ByteSize(content.len() as u64),
        config.url,
        config.bucket,
        path
    );
    if content.len() > 100 * 1024 * 1024 {
        println!("file too large for simple PUT(> 100MB), uploading with multipart upload");
        let progress_bar = ProgressBar::new(content.len() as u64);
        progress_bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
            )
            .unwrap()
            .progress_chars("#>-"));

        let action = CreateMultipartUpload::new(&bucket, Some(&config.credentials), &path);

        let url = action.sign(ONE_HOUR);

        let resp = client.post(url);
        let resp = match resp.send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error creating multipart upload: {}", e);
                exit(1);
            }
        };
        let resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("error creating multipart upload: {}", e);
                exit(1);
            }
        };

        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error creating multipart upload: {}", e);
                exit(1);
            }
        };

        let upload = match CreateMultipartUpload::parse_response(&body) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("error creating multipart upload: {}", e);
                exit(1);
            }
        };

        let (upload_tx, upload_rx) = flume::bounded(8);
        let (etag_tx, etag_rx) = flume::unbounded();
        let runners = (0..num_threads)
            .map(|_| {
                let client = client.clone();
                let bucket = bucket.clone();
                let config = config.clone();
                let path = path.clone();
                let upload = upload.clone();
                spawn_upload_actor(
                    config,
                    bucket,
                    path,
                    client,
                    upload.upload_id().to_string(),
                    upload_rx.clone(),
                    etag_tx.clone(),
                )
            })
            .collect::<Vec<_>>();
        let mut parts = Vec::new();
        for (i, chunk) in content.chunks(chunk_size).enumerate() {
            upload_tx.send((i as u16, chunk.to_vec())).unwrap();
            progress_bar.set_position((i + 1) as u64 * chunk_size as u64);
        }
        drop(upload_tx);
        drop(upload_rx);
        drop(etag_tx);
        for runner in runners {
            runner.await.unwrap();
        }
        while let Ok((i, etag)) = etag_rx.recv() {
            parts.push((i, etag));
        }
        drop(etag_rx);

        parts.sort_by_key(|p| p.0);

        let action = CompleteMultipartUpload::new(
            &bucket,
            Some(&config.credentials),
            &path,
            upload.upload_id(),
            parts.iter().map(|p| p.1.as_str()),
        );
        let url = action.sign(ONE_HOUR);

        let resp = match client.post(url).body(action.body()).send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error completing multipart upload: {}", e);
                exit(1);
            }
        };
        match resp.error_for_status() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("error completing multipart upload: {}", e);
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
                eprintln!("error uploading file: {}", e);
                exit(1);
            }
        };
        match resp.error_for_status() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("error uploading file: {}", e);
                exit(1);
            }
        }
        println!(
            "uploaded file in {:?} ({}/s)",
            now.elapsed(),
            ByteSize((content_len as f64 / now.elapsed().as_secs_f64()) as u64)
        );
    }

    // 2. Get the url of the file
    // -> presigned url

    // 2.1. Create presigned url
    let mut action = GetObject::new(&bucket, Some(&config.credentials), &path);
    action
        .query_mut()
        .insert("response-cache-control", "no-cache, no-store");
    let url = action.sign(Duration::from_secs(config.expires.into()));

    // 2.2. Print url
    println!("\n{}", url);
}
