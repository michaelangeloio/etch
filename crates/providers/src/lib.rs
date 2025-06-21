use futures_util::{Stream, StreamExt};
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use http_body_util::Full;
use hyper::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use sse::EventStream;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

type HttpsClient = hyper_util::client::legacy::Client<HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, http_body_util::Full<hyper::body::Bytes>>;

#[derive(Clone, Debug)]
pub struct AnthropicClient {
    client: HttpsClient,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_only()
            .enable_http1()
            .build();
        
        let client = Client::builder(TokioExecutor::new()).build(https);
        
        Self { client, api_key: api_key.into() }
    }

    pub async fn stream(
        &self,
        request: MessageRequest,
    ) -> Result<impl Stream<Item = Result<StreamEvent, Box<dyn Error + Send + Sync>>>, Box<dyn Error + Send + Sync>> {
        let mut body = serde_json::to_value(&request)?;
        body["stream"] = Value::Bool(true);
        
        let req = Request::builder()
            .method("POST")
            .uri(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(serde_json::to_string(&body)?)))?;

        let resp = self.client.request(req).await?;
        let body = resp.into_body();

        // Use our SSE library to handle the stream
        let event_stream = EventStream::<StreamEvent>::new(body);
        
        // Convert SseError to Box<dyn Error + Send + Sync>
        let mapped_stream = event_stream.map(|result| {
            result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
        });
        
        Ok(mapped_stream)
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


