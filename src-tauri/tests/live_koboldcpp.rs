//! Live integration test against a real KoboldCpp instance.
//! Skipped automatically if no KoboldCpp is reachable at the test endpoint
//! (configure via SAMMY_KOBOOLDCPP_URL env var, default http://localhost:5001).

use sammy_lib::conversation::Role;
use sammy_lib::providers::{
    ChatRequest, HttpKoboldTransport, KoboldCppProvider, KoboldTransport, Provider,
    ProviderMessage, RoutingMode,
};

fn endpoint() -> String {
    std::env::var("SAMMY_KOBOLDCPP_URL")
        .unwrap_or_else(|_| "http://localhost:5001".into())
}

fn koboldcpp_reachable() -> bool {
    let transport = HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(5));
    KoboldTransport::list_models(&transport, &endpoint()).is_ok()
}

#[test]
fn live_koboldcpp_lists_models() {
    if !koboldcpp_reachable() {
        eprintln!("skipping live KoboldCpp test — endpoint not reachable");
        return;
    }
    let transport = HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(10));
    let raw = KoboldTransport::list_models(&transport, &endpoint()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let models: Vec<&str> = v["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert!(
        !models.is_empty(),
        "KoboldCpp should have at least one model loaded"
    );
    println!("KoboldCpp models: {:?}", models);
}

#[test]
fn live_koboldcpp_chat_returns_content() {
    if !koboldcpp_reachable() {
        eprintln!("skipping live KoboldCpp test — endpoint not reachable");
        return;
    }
    // Get the model name
    let transport_models =
        HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(10));
    let raw = KoboldTransport::list_models(&transport_models, &endpoint()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let model = v["data"][0]["id"].as_str().unwrap().to_string();

    let provider = KoboldCppProvider::new(
        endpoint(),
        model.clone(),
        Box::new(HttpKoboldTransport::with_timeout(
            std::time::Duration::from_secs(60),
        )),
    );
    let req = ChatRequest {
        system: "You are a helpful assistant. Reply in one short sentence.".into(),
        messages: vec![ProviderMessage {
            role: Role::User,
            content: "Say hello in exactly three words.".into(),
        }],
        model,
        max_tokens: 64,
        routing: RoutingMode::LocalOnly,
    };
    let resp = Provider::chat(&provider, &req).unwrap();
    println!("KoboldCpp response: {}", resp.content);
    assert!(
        !resp.content.is_empty(),
        "response content must not be empty"
    );
    assert_eq!(resp.provider, "koboldcpp");
    assert!(
        !resp.crossed_to_cloud,
        "local provider must never report cloud crossing"
    );
}
