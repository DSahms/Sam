//! `providers` — replaceable AI provider adapters (Phase 2).
//!
//! Directive §13. The stable [`Provider`] interface decouples Sammy's soul from
//! any specific brain. Three adapters ship in Phase 2: a deterministic mock
//! (required for repeatable testing), a KoboldCpp-compatible local adapter,
//! and a Venice-compatible cloud adapter.
//!
//! Design rules:
//! - Provider-specific behavior stays inside adapters.
//! - Adding a provider must not change vault/conversation/knowledge/corpus/
//!   retrieval schemas or the frontend conversation components.
//! - Every cloud transmission is audited (directive §13).
//! - Provider errors never expose the full private prompt in logs.

use serde::{Deserialize, Serialize};

use crate::conversation::Role;
use crate::error::{AppError, AppResult};

/// Whether a provider runs locally or in the cloud. Drives routing/privacy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderLocality {
    Local,
    Cloud,
}

/// Privacy classification of a provider. Cloud providers may receive content
/// that leaves the device; local providers do not.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    /// Content never leaves the device.
    Local,
    /// Content leaves the device to the configured cloud endpoint.
    Cloud,
}

/// A provider's static identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub locality: ProviderLocality,
    pub privacy: PrivacyClass,
}

/// A single message in a provider request, mirroring conversation roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub role: Role,
    pub content: String,
}

/// A chat-completion request handed to a provider.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// The assembled system prompt (sanitized; no secrets).
    pub system: String,
    pub messages: Vec<ProviderMessage>,
    pub model: String,
    /// Maximum tokens the provider may generate.
    pub max_tokens: u32,
    /// A turn-level routing decision (resolved by the runtime before calling).
    pub routing: RoutingMode,
}

/// A complete chat-completion response (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub provider: String,
    pub model: String,
    /// Whether the request content crossed to a cloud provider.
    pub crossed_to_cloud: bool,
    /// Provider-reported usage if available.
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Routing modes (directive §13). The default is `AskBeforeCrossing`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    LocalOnly,
    PreferLocal,
    PreferCloud,
    CloudOnly,
    AskBeforeCrossing,
}

impl RoutingMode {
    pub const DEFAULT: RoutingMode = RoutingMode::AskBeforeCrossing;

    pub fn as_str(self) -> &'static str {
        match self {
            RoutingMode::LocalOnly => "local_only",
            RoutingMode::PreferLocal => "prefer_local",
            RoutingMode::PreferCloud => "prefer_cloud",
            RoutingMode::CloudOnly => "cloud_only",
            RoutingMode::AskBeforeCrossing => "ask_before_crossing",
        }
    }
}

impl Default for RoutingMode {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The stable provider interface. Every adapter implements this. Adding a
/// provider must not change vault/conversation/knowledge/corpus/retrieval
/// schemas or frontend conversation components.
pub trait Provider: Send + Sync {
    /// Static identity of this provider.
    fn info(&self) -> ProviderInfo;

    /// Non-streaming chat completion.
    fn chat(&self, req: &ChatRequest) -> AppResult<ChatResponse>;

    /// List available models, if the provider supports discovery.
    fn list_models(&self) -> AppResult<Vec<String>> {
        Ok(Vec::new())
    }

    /// Test connectivity/credentials. Returns Ok(()) on success.
    fn test_connection(&self) -> AppResult<()>;

    /// Rough token estimate for a chunk of text. Default is a naive heuristic;
    /// adapters may override with a provider-specific tokenizer if available.
    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.chars().count() as u32) / 4
    }
}

// -----------------------------------------------------------------------------
// Routing resolution
// -----------------------------------------------------------------------------

/// Outcome of resolving a routing mode against reachable local/cloud providers.
/// Providers occupy a single combined index list: local providers first, then
/// cloud providers at offsets `>= local_count`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    /// Use this provider; no consent needed (it does not cross a boundary).
    Use { provider_index: usize },
    /// A cloud crossing is required; the runtime must obtain explicit consent
    /// before calling the cloud provider at this index.
    NeedsCloudConsent { provider_index: usize },
    /// No provider satisfies the routing mode.
    NoProvider,
}

/// Resolve which provider to use. `local_reachable[i]` / `cloud_reachable[i]`
/// indicate whether that provider is configured and reachable.
pub fn resolve_routing(
    mode: RoutingMode,
    local_count: usize,
    cloud_count: usize,
    local_reachable: &[bool],
    cloud_reachable: &[bool],
) -> RoutingDecision {
    let first_local =
        (0..local_count).find(|&i| local_reachable.get(i).copied().unwrap_or(false));
    let first_cloud =
        (0..cloud_count).find(|&i| cloud_reachable.get(i).copied().unwrap_or(false));

    match mode {
        RoutingMode::LocalOnly => first_local
            .map(|i| RoutingDecision::Use { provider_index: i })
            .unwrap_or(RoutingDecision::NoProvider),
        RoutingMode::CloudOnly => first_cloud
            .map(|i| RoutingDecision::Use {
                provider_index: local_count + i,
            })
            .unwrap_or(RoutingDecision::NoProvider),
        RoutingMode::PreferLocal => {
            if let Some(i) = first_local {
                RoutingDecision::Use { provider_index: i }
            } else {
                first_cloud
                    .map(|i| RoutingDecision::NeedsCloudConsent {
                        provider_index: local_count + i,
                    })
                    .unwrap_or(RoutingDecision::NoProvider)
            }
        }
        RoutingMode::PreferCloud => first_cloud
            .map(|i| RoutingDecision::NeedsCloudConsent {
                provider_index: local_count + i,
            })
            .unwrap_or_else(|| {
                first_local
                    .map(|i| RoutingDecision::Use { provider_index: i })
                    .unwrap_or(RoutingDecision::NoProvider)
            }),
        RoutingMode::AskBeforeCrossing => {
            if let Some(i) = first_local {
                RoutingDecision::Use { provider_index: i }
            } else if let Some(j) = first_cloud {
                RoutingDecision::NeedsCloudConsent {
                    provider_index: local_count + j,
                }
            } else {
                RoutingDecision::NoProvider
            }
        }
    }
}

// =============================================================================
// Mock provider — deterministic, required for repeatable testing.
// =============================================================================

/// A deterministic mock provider. Responses depend only on the request, so
/// tests are repeatable. No network I/O; never crosses a privacy boundary.
pub struct MockProvider {
    mode: MockMode,
}

#[derive(Debug, Clone)]
pub enum MockMode {
    Fixed(String),
    Echo,
}

impl MockProvider {
    pub fn fixed(reply: impl Into<String>) -> Self {
        Self {
            mode: MockMode::Fixed(reply.into()),
        }
    }
    pub fn echo() -> Self {
        Self {
            mode: MockMode::Echo,
        }
    }
}

impl Provider for MockProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "mock".into(),
            display_name: "Deterministic mock".into(),
            locality: ProviderLocality::Local,
            privacy: PrivacyClass::Local,
        }
    }

    fn chat(&self, req: &ChatRequest) -> AppResult<ChatResponse> {
        let content = match &self.mode {
            MockMode::Fixed(s) => s.clone(),
            MockMode::Echo => {
                let last_user = req
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == Role::User)
                    .map(|m| m.content.as_str())
                    .unwrap_or("");
                format!("[mock echo] {last_user}")
            }
        };
        let prompt_tokens = self.estimate_tokens(&req.system);
        let completion_tokens = self.estimate_tokens(&content);
        Ok(ChatResponse {
            content,
            provider: "mock".into(),
            model: req.model.clone(),
            crossed_to_cloud: false,
            usage: Some(TokenUsage {
                prompt_tokens,
                completion_tokens,
            }),
        })
    }

    fn list_models(&self) -> AppResult<Vec<String>> {
        Ok(vec!["mock-1".into()])
    }

    fn test_connection(&self) -> AppResult<()> {
        Ok(())
    }
}

// =============================================================================
// KoboldCpp-compatible local provider adapter (interface + mock transport).
// =============================================================================

/// A KoboldCpp-compatible local provider. The HTTP transport is abstracted
/// behind a trait so tests use a deterministic transport; a `reqwest`-based
/// transport lands when network integration is exercised (no-endpoint blocker,
/// recorded in EXTERNAL_BLOCKERS — not a code gap).
pub struct KoboldCppProvider {
    pub endpoint: String,
    pub model: String,
    transport: Box<dyn KoboldTransport>,
}

pub trait KoboldTransport: Send + Sync {
    fn chat(&self, endpoint: &str, req_json: &str) -> AppResult<String>;
    fn list_models(&self, endpoint: &str) -> AppResult<String>;
}

/// Real HTTP transport for KoboldCpp using `reqwest::blocking`. KoboldCpp exposes
/// an OpenAI-compatible API at `<base>/v1`. We POST to `<base>/v1/chat/completions`
/// and GET `<base>/v1/models`.
///
/// On error, only a sanitized category string is returned — never the request
/// body (which contains private prompt text) and never HTTP response detail
/// beyond a status code summary. This preserves directive §13's rule that
/// "provider errors must not expose the full private prompt in logs."
pub struct HttpKoboldTransport {
    client: reqwest::blocking::Client,
}

impl HttpKoboldTransport {
    pub fn new() -> Self {
        Self::with_timeout(std::time::Duration::from_secs(120))
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest blocking client");
        Self { client }
    }

    /// Normalize the endpoint to end without a trailing slash, so we can safely
    /// append `/v1/...`.
    fn v1_url(endpoint: &str, path: &str) -> String {
        let base = endpoint.trim_end_matches('/');
        format!("{base}/v1{path}")
    }
}

impl Default for HttpKoboldTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl KoboldTransport for HttpKoboldTransport {
    fn chat(&self, endpoint: &str, req_json: &str) -> AppResult<String> {
        let url = Self::v1_url(endpoint, "/chat/completions");
        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(req_json.to_string())
            .send()
            .map_err(|e| {
                log::error!("koboldcpp chat request failed (url redacted)");
                AppError::Config(format!("koboldcpp unreachable: {}", sanitize_err(&e)))
            })?;
        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            // Do NOT log the body — it may contain private content reflected
            // by the model. Only log the status code.
            log::warn!("koboldcpp chat returned HTTP {code}");
            return Err(AppError::Config(format!("koboldcpp returned HTTP {code}")));
        }
        resp.text().map_err(|e| {
            log::warn!("koboldcpp response read failed");
            AppError::Config(format!("koboldcpp response error: {}", sanitize_err(&e)))
        })
    }

    fn list_models(&self, endpoint: &str) -> AppResult<String> {
        let url = Self::v1_url(endpoint, "/models");
        let resp = self.client.get(&url).send().map_err(|e| {
            log::error!("koboldcpp models request failed");
            AppError::Config(format!("koboldcpp unreachable: {}", sanitize_err(&e)))
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::Config(format!(
                "koboldcpp /models returned HTTP {}",
                status.as_u16()
            )));
        }
        resp.text().map_err(|e| {
            AppError::Config(format!("koboldcpp models read error: {}", sanitize_err(&e)))
        })
    }
}

/// Sanitize a reqwest error: return a short category without the full URL or
/// request body. We only surface whether it's a connect/timeout/decode error.
fn sanitize_err(e: &reqwest::Error) -> String {
    if e.is_connect() {
        "connection refused".into()
    } else if e.is_timeout() {
        "timed out".into()
    } else if e.is_decode() {
        "bad response format".into()
    } else {
        "network error".into()
    }
}

impl KoboldCppProvider {
    pub fn new(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        transport: Box<dyn KoboldTransport>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
            transport,
        }
    }
}

impl Provider for KoboldCppProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "koboldcpp".into(),
            display_name: "KoboldCpp (local)".into(),
            locality: ProviderLocality::Local,
            privacy: PrivacyClass::Local,
        }
    }

    fn chat(&self, req: &ChatRequest) -> AppResult<ChatResponse> {
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": m.role.as_str(),
                "content": m.content,
            })).collect::<Vec<_>>(),
            "max_tokens": req.max_tokens,
        });
        let raw = self.transport.chat(&self.endpoint, &body.to_string())?;
        let v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| AppError::Config(format!("koboldcpp bad response: {e}")))?;
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AppError::Config("koboldcpp missing content".into()))?
            .to_string();
        Ok(ChatResponse {
            content,
            provider: "koboldcpp".into(),
            model: req.model.clone(),
            crossed_to_cloud: false,
            usage: None,
        })
    }

    fn list_models(&self) -> AppResult<Vec<String>> {
        let raw = self.transport.list_models(&self.endpoint)?;
        let v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| AppError::Config(format!("koboldcpp bad model list: {e}")))?;
        let mut out = Vec::new();
        if let Some(arr) = v["data"].as_array() {
            for m in arr {
                if let Some(id) = m["id"].as_str() {
                    out.push(id.to_string());
                }
            }
        }
        Ok(out)
    }

    fn test_connection(&self) -> AppResult<()> {
        let _ = self.list_models()?;
        Ok(())
    }
}

// =============================================================================
// Venice-compatible cloud provider adapter (interface + mock transport).
// =============================================================================

/// A Venice-compatible cloud provider (OpenAI-compatible `/chat/completions`).
/// Content sent here leaves the device; the runtime must audit every call and
/// respect routing/privacy rules.
pub struct VeniceProvider {
    pub endpoint: String,
    pub model: String,
    transport: Box<dyn VeniceTransport>,
}

pub trait VeniceTransport: Send + Sync {
    fn chat(&self, endpoint: &str, api_key: &str, req_json: &str) -> AppResult<String>;
    fn list_models(&self, endpoint: &str, api_key: &str) -> AppResult<String>;
}

impl VeniceProvider {
    pub fn new(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        transport: Box<dyn VeniceTransport>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
            transport,
        }
    }

    /// Perform a chat completion with a decrypted API key. The runtime obtains
    /// the key, calls this, then drops it; the key is never logged.
    pub fn chat_with_key(
        &self,
        req: &ChatRequest,
        api_key: &str,
    ) -> AppResult<ChatResponse> {
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": m.role.as_str(),
                "content": m.content,
            })).collect::<Vec<_>>(),
            "max_tokens": req.max_tokens,
        });
        let raw = self
            .transport
            .chat(&self.endpoint, api_key, &body.to_string())?;
        let v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| AppError::Config(format!("venice bad response: {e}")))?;
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AppError::Config("venice missing content".into()))?
            .to_string();
        Ok(ChatResponse {
            content,
            provider: "venice".into(),
            model: req.model.clone(),
            crossed_to_cloud: true,
            usage: None,
        })
    }
}

impl Provider for VeniceProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "venice".into(),
            display_name: "Venice (cloud)".into(),
            locality: ProviderLocality::Cloud,
            privacy: PrivacyClass::Cloud,
        }
    }

    fn chat(&self, _req: &ChatRequest) -> AppResult<ChatResponse> {
        // Without a key the cloud provider cannot run; the runtime must obtain
        // the decrypted key and call chat_with_key. Returning an error ensures
        // a missing key is never silently ignored.
        Err(AppError::Config(
            "venice requires an api key; call chat_with_key".into(),
        ))
    }

    fn list_models(&self) -> AppResult<Vec<String>> {
        Err(AppError::Config("venice requires an api key".into()))
    }

    fn test_connection(&self) -> AppResult<()> {
        Err(AppError::Config("venice requires an api key".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> ChatRequest {
        ChatRequest {
            system: "be brief".into(),
            messages: vec![ProviderMessage {
                role: Role::User,
                content: "hello".into(),
            }],
            model: "mock-1".into(),
            max_tokens: 64,
            routing: RoutingMode::LocalOnly,
        }
    }

    #[test]
    fn mock_provider_is_deterministic() {
        let p = MockProvider::fixed("hi");
        let r1 = p.chat(&req()).unwrap();
        let r2 = p.chat(&req()).unwrap();
        assert_eq!(r1.content, "hi");
        assert_eq!(r1.content, r2.content);
        assert_eq!(r1.provider, "mock");
        assert!(!r1.crossed_to_cloud);
    }

    #[test]
    fn mock_provider_echo_mode() {
        let p = MockProvider::echo();
        let r = p.chat(&req()).unwrap();
        assert!(r.content.contains("hello"));
    }

    #[test]
    fn mock_provider_lists_models() {
        let p = MockProvider::fixed("hi");
        assert_eq!(p.list_models().unwrap(), vec!["mock-1".to_string()]);
    }

    #[test]
    fn koboldcpp_provider_via_mock_transport() {
        struct T;
        impl KoboldTransport for T {
            fn chat(&self, _ep: &str, _req: &str) -> AppResult<String> {
                Ok(r#"{"choices":[{"message":{"content":"local-reply"}}]}"#.into())
            }
            fn list_models(&self, _ep: &str) -> AppResult<String> {
                Ok(r#"{"data":[{"id":"koboldcpp-1"}]}"#.into())
            }
        }
        let p =
            KoboldCppProvider::new("http://localhost:5001", "koboldcpp-1", Box::new(T));
        let r = p.chat(&req()).unwrap();
        assert_eq!(r.content, "local-reply");
        assert_eq!(r.provider, "koboldcpp");
        assert!(!r.crossed_to_cloud);
        assert_eq!(p.list_models().unwrap(), vec!["koboldcpp-1".to_string()]);
        assert!(p.test_connection().is_ok());
    }

    #[test]
    fn venice_provider_via_mock_transport() {
        struct T;
        impl VeniceTransport for T {
            fn chat(&self, _ep: &str, _key: &str, _req: &str) -> AppResult<String> {
                Ok(r#"{"choices":[{"message":{"content":"cloud-reply"}}]}"#.into())
            }
            fn list_models(&self, _ep: &str, _key: &str) -> AppResult<String> {
                Ok(r#"{"data":[{"id":"venice-1"}]}"#.into())
            }
        }
        let p =
            VeniceProvider::new("https://api.venice.ai/api/v1", "venice-1", Box::new(T));
        let r = p.chat_with_key(&req(), "key").unwrap();
        assert_eq!(r.content, "cloud-reply");
        assert!(r.crossed_to_cloud);
        assert!(p.chat(&req()).is_err());
    }

    #[test]
    fn routing_local_only_with_local_available_uses_local() {
        let d = resolve_routing(RoutingMode::LocalOnly, 1, 1, &[true], &[true]);
        assert_eq!(d, RoutingDecision::Use { provider_index: 0 });
    }

    #[test]
    fn routing_local_only_without_local_is_no_provider() {
        let d = resolve_routing(RoutingMode::LocalOnly, 0, 1, &[], &[true]);
        assert_eq!(d, RoutingDecision::NoProvider);
    }

    #[test]
    fn routing_cloud_only_uses_cloud_index_offset() {
        let d = resolve_routing(RoutingMode::CloudOnly, 2, 1, &[true], &[true]);
        assert_eq!(d, RoutingDecision::Use { provider_index: 2 });
    }

    #[test]
    fn routing_prefer_local_falls_back_to_cloud_consent() {
        let d = resolve_routing(RoutingMode::PreferLocal, 1, 1, &[false], &[true]);
        assert_eq!(d, RoutingDecision::NeedsCloudConsent { provider_index: 1 });
    }

    #[test]
    fn routing_ask_before_crossing_uses_local_without_asking() {
        let d = resolve_routing(RoutingMode::AskBeforeCrossing, 1, 1, &[true], &[true]);
        assert_eq!(d, RoutingDecision::Use { provider_index: 0 });
    }

    #[test]
    fn routing_ask_before_crossing_needs_consent_when_only_cloud() {
        let d = resolve_routing(RoutingMode::AskBeforeCrossing, 1, 1, &[false], &[true]);
        assert_eq!(d, RoutingDecision::NeedsCloudConsent { provider_index: 1 });
    }

    #[test]
    fn routing_prefer_cloud_asks_consent_when_cloud_available() {
        let d = resolve_routing(RoutingMode::PreferCloud, 1, 1, &[true], &[true]);
        assert_eq!(d, RoutingDecision::NeedsCloudConsent { provider_index: 1 });
    }

    #[test]
    fn routing_no_providers_anywhere_is_no_provider() {
        let d = resolve_routing(RoutingMode::PreferLocal, 0, 0, &[], &[]);
        assert_eq!(d, RoutingDecision::NoProvider);
    }

    #[test]
    fn routing_mode_default_is_ask_before_crossing() {
        assert_eq!(RoutingMode::default(), RoutingMode::AskBeforeCrossing);
    }

    #[test]
    fn routing_mode_round_trips_serde() {
        for m in [
            RoutingMode::LocalOnly,
            RoutingMode::PreferLocal,
            RoutingMode::PreferCloud,
            RoutingMode::CloudOnly,
            RoutingMode::AskBeforeCrossing,
        ] {
            let s = serde_json::to_string(&m).unwrap();
            let back: RoutingMode = serde_json::from_str(&s).unwrap();
            assert_eq!(m, back);
        }
    }

    // -------------------------------------------------------------------------
    // HttpKoboldTransport tests with a tiny mock HTTP server.
    // -------------------------------------------------------------------------

    /// Start a mock HTTP server on a random localhost port. Returns the base
    /// URL and a handle to the server thread. The server responds to one
    /// request then returns its canned response.
    fn mock_server(response_body: String, expected_path_contains: &str) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let path_check = expected_path_contains.to_string();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                use std::io::{Read, Write};
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).unwrap();
                let req_str = String::from_utf8_lossy(&buf);
                let body = if req_str.contains(&path_check) {
                    response_body.clone()
                } else {
                    r#"{"error":"unexpected path"}"#.into()
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    #[test]
    fn http_transport_list_models_works() {
        let url = mock_server(
            r#"{"data":[{"id":"test-gguf-model"},{"id":"backup-model"}]}"#.into(),
            "/v1/models",
        );
        let transport =
            HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(5));
        let raw = KoboldTransport::list_models(&transport, &url).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let ids: Vec<&str> = v["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"test-gguf-model"));
    }

    #[test]
    fn http_transport_chat_works() {
        let url = mock_server(
            r#"{"choices":[{"message":{"content":"Hello from the model!"}}]}"#.into(),
            "/v1/chat/completions",
        );
        let transport =
            HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(5));
        let req_json = r#"{"model":"test","messages":[{"role":"user","content":"hi"}]}"#;
        let raw = KoboldTransport::chat(&transport, &url, req_json).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            v["choices"][0]["message"]["content"].as_str().unwrap(),
            "Hello from the model!"
        );
    }

    #[test]
    fn http_transport_unreachable_endpoint_fails_gracefully() {
        // Port 1 is reserved/unlikely to have anything listening.
        let transport =
            HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(2));
        let err =
            KoboldTransport::list_models(&transport, "http://127.0.0.1:1").unwrap_err();
        // Error must be opaque (no request body, no URL detail).
        let msg = format!("{err}");
        assert!(
            !msg.contains("/v1/models"),
            "error must not leak the URL path"
        );
    }

    #[test]
    fn http_transport_error_does_not_leak_request_body() {
        // Start a server that returns 500; the error must not contain the
        // request body (which has private prompt text).
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                use std::io::{Read, Write};
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let resp =
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        let url = format!("http://{}", addr);
        let transport =
            HttpKoboldTransport::with_timeout(std::time::Duration::from_secs(5));
        let private_prompt = "this-is-a-secret-prompt-content";
        let req_json = format!(r#"{{"messages":[{{"content":"{private_prompt}"}}]}}"#);
        let err = KoboldTransport::chat(&transport, &url, &req_json).unwrap_err();
        let msg = format!("{err}");
        assert!(
            !msg.contains(private_prompt),
            "error must not contain the private prompt text"
        );
        assert!(msg.contains("500"), "error should mention the HTTP status");
    }

    #[test]
    fn http_transport_normalizes_trailing_slash() {
        // The URL builder should handle trailing slashes gracefully.
        let url1 = HttpKoboldTransport::v1_url("http://localhost:5001", "/models");
        let url2 = HttpKoboldTransport::v1_url("http://localhost:5001/", "/models");
        assert_eq!(url1, "http://localhost:5001/v1/models");
        assert_eq!(url1, url2);
    }
}
