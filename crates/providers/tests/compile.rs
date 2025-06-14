use providers::*;

#[tokio::test]
async fn compile_client() {
    let _client = AnthropicClient::new("test");
    let req = MessageRequest {
        model: "claude-3-opus-20240229".into(),
        messages: vec![Message { role: "user".into(), content: "hi".into() }],
        max_tokens: 10,
        temperature: None,
    };
    let _ = _client.stream(req); // compile check
}
