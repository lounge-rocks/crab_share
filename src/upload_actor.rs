use std::process::exit;

use reqwest::{header::ETAG, Client};
use rusty_s3::{actions::UploadPart, Bucket, S3Action};

use crate::{config::Config, ONE_HOUR};

struct UploadActor {
    client: Client,
    bucket: Bucket,
    upload_id: String,
    config: Config,
    path: String,
}

impl UploadActor {
    async fn new(
        config: Config,
        bucket: Bucket,
        path: String,
        client: Client,
        upload_id: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            client,
            bucket,
            upload_id,
            config,
            path,
        })
    }

    async fn upload_part(&self, part_number: u16, chunk: &[u8]) -> String {
        let action = UploadPart::new(
            &self.bucket,
            Some(&self.config.credentials),
            &self.path,
            part_number + 1,
            &self.upload_id,
        );
        let url = action.sign(ONE_HOUR);
        let resp = match self.client.put(url).body(chunk.to_vec()).send().await {
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
}

pub fn spawn_upload_actor(
    config: Config,
    bucket: Bucket,
    path: String,
    client: Client,
    upload_id: String,
    upload_rx: flume::Receiver<(u16, Vec<u8>)>,
    etag_tx: flume::Sender<(u16, String)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let actor = UploadActor::new(config, bucket, path, client, upload_id)
            .await
            .unwrap();
        while let Ok((part_number, chunk)) = upload_rx.recv() {
            let etag = actor.upload_part(part_number, &chunk).await;
            // println!("uploaded part {} after {:?}", part_number, now.elapsed());
            etag_tx.send((part_number, etag)).unwrap();
        }
    })
}
