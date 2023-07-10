// list bucket and filter out files that are expired

use std::process::exit;

use reqwest::Client;
use rusty_s3::actions::{ListObjectsV2, S3Action};
use rusty_s3::Bucket;

use crate::ONE_HOUR;

pub async fn purge(config: &crate::config::Config, bucket: &Bucket) {
    let client = Client::new();

    let mut action = ListObjectsV2::new(bucket, Some(&config.credentials));
    let url: reqwest::Url = action.sign(ONE_HOUR);
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error listing bucket: {}", e);
            exit(1);
        }
    };
    let resp = match resp.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("error listing bucket: {}", e);
            exit(1);
        }
    };
    let resp = match resp.text().await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("error listing bucket: {}", e);
            exit(1);
        }
    };
    let resp = match ListObjectsV2::parse_response(&resp) {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("error listing bucket: {}", e);
            exit(1);
        }
    };
    let mut files = resp.contents;

    while let Some(ref continuation_token) = resp.next_continuation_token {
        action
            .query_mut()
            .insert("continuation-token", continuation_token);
        let url: reqwest::Url = action.sign(ONE_HOUR);
        let resp = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error listing bucket: {}", e);
                exit(1);
            }
        };
        let resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("error listing bucket: {}", e);
                exit(1);
            }
        };
        let resp = match resp.text().await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("error listing bucket: {}", e);
                exit(1);
            }
        };
        let resp = match ListObjectsV2::parse_response(&resp) {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("error listing bucket: {}", e);
                exit(1);
            }
        };
        files.extend(resp.contents);
    }
    // extract only the key from each file
    let files: Vec<_> = files
        .into_iter()
        .map(|f| f.key)
        // decode the timestamp from the uuid
        .map(|k| {
            let mut parts = k.split('/');
            // first part is the uuid
            let ulid = parts.next().unwrap();
            let ulid = ulid.parse::<ulid::Ulid>().unwrap();
            let expiry_timestamp = ulid.timestamp_ms();
            (k, expiry_timestamp)
        })
        .collect();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut files_to_delete = vec![];
    for file in files {
        let (key, timestamp) = file;
        if timestamp < now {
            files_to_delete.push(key);
        }
    }

    for file in files_to_delete {
        let action = rusty_s3::actions::DeleteObject::new(bucket, Some(&config.credentials), &file);
        let url = action.sign(ONE_HOUR);
        let resp = match client.delete(url).send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error deleting file: {}", e);
                exit(1);
            }
        };
        let _resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("error deleting file: {}", e);
                exit(1);
            }
        };
        println!("deleted expired file: {}", file);
    }
}
