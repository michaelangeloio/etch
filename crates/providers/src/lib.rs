use async_stream::try_stream;
use futures_core::stream::Stream;
use reqwest::{Client, Response};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

#[derive(Clone, Debug)]
pub struct AnthropicClient {
    client: Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self { client: Client::new(), api_key: api_key.into() }
    }

    pub async fn stream(
        &self,
        request: MessageRequest,
    ) -> reqwest::Result<impl Stream<Item = reqwest::Result<StreamEvent>>> {
        let mut body = serde_json::to_value(&request).unwrap();
        body["stream"] = Value::Bool(true);
        let resp = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await?;
        Ok(parse_sse(resp))
    }
}

#[derive(Serialize, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Debug)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    #[serde(flatten)]
    pub inner: Value,
}

fn parse_sse(resp: Response) -> impl Stream<Item = reqwest::Result<StreamEvent>> {
    try_stream! {
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            buf.push_str(&text);
            while let Some(idx) = buf.find("\n\n") {
                let event_str = buf[..idx].to_string();
                buf.replace_range(..idx + 2, "");
                let event = event_str.trim();
                if event.starts_with("data: ") {
                    let data = &event[6..];
                    if data == "[DONE]" { return; }
                    let evt: StreamEvent = serde_json::from_str(data).unwrap();
                    yield evt;
                }
            }
        }
    }
}


