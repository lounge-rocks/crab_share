// list bucket and filter out files that are expired

use std::process::exit;

use percent_encoding::percent_decode_str;
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
    let mut continuation_token = resp.next_continuation_token;

    while let Some(token) = continuation_token.take() {
        action.query_mut().insert("continuation-token", token);
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
        continuation_token = resp.next_continuation_token;
    }
    // extract only the key from each file
    let files: Vec<_> = files
        .into_iter()
        // ListObjectsV2 requests encoding-type=url, so returned keys must be
        // decoded before they are passed to DeleteObject.
        .map(|f| decode_listed_key(f.key))
        // decode the timestamp from the uuid
        .flat_map(|k| {
            let mut parts = k.split('/');
            // first part is the uuid
            let ulid = match parts.next() {
                Some(ulid) => ulid,
                None => return None,
            };
            let ulid = match ulid.parse::<ulid::Ulid>() {
                Ok(ulid) => ulid,
                Err(_) => return None,
            };
            let expiry_timestamp = ulid.timestamp_ms();
            Some((k, expiry_timestamp))
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

fn decode_listed_key(key: String) -> String {
    percent_decode_str(&key)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rusty_s3::actions::{DeleteObject, S3Action};
    use rusty_s3::{Bucket, UrlStyle};

    use super::decode_listed_key;

    #[test]
    fn decodes_url_encoded_s3_keys() {
        let key = "01KK3RNR08TGMCJ9E1W0949R1Y/100%25%20complete/%E2%9C%93.pdf";

        assert_eq!(
            decode_listed_key(key.to_string()),
            "01KK3RNR08TGMCJ9E1W0949R1Y/100% complete/✓.pdf"
        );
    }

    #[test]
    fn leaves_invalid_utf8_encoding_unchanged() {
        let key = "01KK3RNR08TGMCJ9E1W0949R1Y/file%FF.pdf";

        assert_eq!(decode_listed_key(key.to_string()), key);
    }

    #[test]
    fn signs_the_decoded_key_for_deletion() {
        let bucket = Bucket::new(
            "https://s3.example.com".parse().unwrap(),
            UrlStyle::Path,
            "bucket",
            "region",
        )
        .unwrap();
        let key =
            decode_listed_key("01KK3RNR08TGMCJ9E1W0949R1Y/Part%20lot100-2%25.pdf".to_string());
        let action = DeleteObject::new(&bucket, None, &key);

        assert_eq!(
            action.sign(Duration::from_secs(60)).as_str(),
            "https://s3.example.com/bucket/01KK3RNR08TGMCJ9E1W0949R1Y/Part%20lot100-2%25.pdf"
        );
    }
}
