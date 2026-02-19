use grok_chat::api::{ChatMessage, ChatRequest, ChatResponse, ModelsResponse, XaiClient};
use mockito::{Matcher, Server};
use reqwest::Method;

#[tokio::test]
async fn chat_round_trip() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/chat/completions")
        .match_header("Authorization", Matcher::Regex("Bearer test-key.*".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "choices": [{
                    "message": {"role": "assistant", "content": "Hello back!"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
            }"#,
        )
        .create_async()
        .await;

    let client = XaiClient::with_base_url("test-key".into(), server.url());
    let req = ChatRequest::new("test-model", vec![ChatMessage::user("hello")]);
    let resp: ChatResponse = client
        .request(Method::POST, "/chat/completions", Some(&req))
        .await
        .expect("request should succeed");

    assert_eq!(resp.choices.len(), 1);
    assert_eq!(
        resp.choices[0].message.content.as_deref(),
        Some("Hello back!")
    );
    assert_eq!(resp.usage.unwrap().total_tokens, 8);
    mock.assert_async().await;
}

#[tokio::test]
async fn http_error_returns_api_error() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(429)
        .with_body(r#"{"error": "rate limited"}"#)
        .create_async()
        .await;

    let client = XaiClient::with_base_url("test-key".into(), server.url());
    let req = ChatRequest::new("test-model", vec![ChatMessage::user("hello")]);
    let result = client
        .request::<_, ChatResponse>(Method::POST, "/chat/completions", Some(&req))
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("429"), "should contain status code: {err}");
    mock.assert_async().await;
}

#[tokio::test]
async fn malformed_json_response() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not valid json")
        .create_async()
        .await;

    let client = XaiClient::with_base_url("test-key".into(), server.url());
    let req = ChatRequest::new("test-model", vec![ChatMessage::user("hello")]);
    let result = client
        .request::<_, ChatResponse>(Method::POST, "/chat/completions", Some(&req))
        .await;

    assert!(result.is_err());
    mock.assert_async().await;
}

#[tokio::test]
async fn list_models() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/models")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": [
                    {"id": "grok-3", "owned_by": "xai"},
                    {"id": "grok-3-mini"}
                ]
            }"#,
        )
        .create_async()
        .await;

    let client = XaiClient::with_base_url("test-key".into(), server.url());
    let resp: ModelsResponse = client
        .request::<(), ModelsResponse>(Method::GET, "/models", None)
        .await
        .expect("should succeed");

    assert_eq!(resp.data.len(), 2);
    assert_eq!(resp.data[0].id, "grok-3");
    assert_eq!(resp.data[1].owned_by, None);
    mock.assert_async().await;
}
