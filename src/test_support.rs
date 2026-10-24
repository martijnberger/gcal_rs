use std::{collections::BTreeMap, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};

use crate::GCalClient;

#[derive(Debug)]
pub(crate) struct ObservedRequest {
    pub method: String,
    pub target: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

/// Ordered local responses; returns captured real HTTP requests for behavior assertions.
pub(crate) async fn fixture(
    responses: Vec<(u16, &'static str)>,
) -> (Arc<GCalClient>, JoinHandle<Vec<ObservedRequest>>) {
    fixture_with_content_type(responses, "application/json").await
}

pub(crate) async fn fixture_with_content_type(
    responses: Vec<(u16, &'static str)>,
    content_type: &'static str,
) -> (Arc<GCalClient>, JoinHandle<Vec<ObservedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = url::Url::parse(&format!(
        "http://{}/calendar/v3/",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut first = String::new();
            reader.read_line(&mut first).await.unwrap();
            let mut words = first.split_whitespace();
            let method = words.next().unwrap().to_string();
            let target = words.next().unwrap().to_string();
            let mut headers = BTreeMap::new();
            loop {
                let mut line = String::new();
                assert_ne!(reader.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (key, value) = line.split_once(':').unwrap();
                headers.insert(key.to_ascii_lowercase(), value.trim().to_string());
            }
            let length = headers
                .get("content-length")
                .map_or(0, |length| length.parse().unwrap());
            let mut request_body = vec![0; length];
            reader.read_exact(&mut request_body).await.unwrap();
            requests.push(ObservedRequest {
                method,
                target,
                headers,
                body: request_body,
            });
            let reason = reqwest::StatusCode::from_u16(status)
                .unwrap()
                .canonical_reason()
                .unwrap_or("Response");
            stream.write_all(format!("HTTP/1.1 {status} {reason}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
        requests
    });
    (GCalClient::for_test(base), task)
}
