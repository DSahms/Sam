//! Read-only adapter to the external Personal Knowledge Corpus (PKC).
//!
//! This is **not** Sammy's internal vault corpus (`corpus` / `retrieval`).
//! PKC is a separate durable-knowledge product. Sammy queries it through the
//! same authorized Python gateway the PKC Reference Client uses, then sanitizes
//! the conversational payload before any model sees it.
//!
//! Constraints for this vertical slice:
//! - feature-gated (`pkc_enabled`, default off)
//! - read-only: no durable memory writes, no corpus mutation
//! - unauthorized / unavailable evidence never reaches the model
//! - PKC bookkeeping must not enter user-facing or model-facing text
//! - no cloud path: callers skip this adapter when the turn is cloud-bound

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

use crate::grounding;
use crate::settings::ProviderConfig;

const BRIDGE_VERSION: &str = "1.0.0";
const CONSUMER_APPLICATION: &str = "sammy";

/// Conversational payload allowed into the prompt, or `None` to degrade safely.
pub fn authorized_payload_for_turn(cfg: &ProviderConfig, question: &str) -> Option<String> {
    if !cfg.pkc_enabled {
        return None;
    }
    if cfg.pkc_source_id.trim().is_empty() || cfg.pkc_bridge_script.trim().is_empty() {
        return None;
    }
    let raw = invoke_bridge(cfg, question)?;
    conversational_payload_from_bridge_json(&raw)
}

/// Parse a PKC Reference Client-shaped bridge response. Returns conversational
/// text only when authorized, materialized, and free of bookkeeping.
pub fn conversational_payload_from_bridge_json(raw: &str) -> Option<String> {
    let value: Value = serde_json::from_str(raw).ok()?;
    if value.get("bridge_version")?.as_str()? != BRIDGE_VERSION {
        return None;
    }
    if value.get("ok")?.as_bool() != Some(true) {
        return None;
    }
    if value.get("available")?.as_bool() != Some(true) {
        return None;
    }
    let payload = value.get("payload")?;
    if payload.get("authorized")?.as_bool() != Some(true) {
        return None;
    }
    if payload.get("canonical_hash_verified")?.as_bool() != Some(true) {
        return None;
    }
    if payload.get("protected_content_materialized")?.as_bool() != Some(true) {
        return None;
    }
    let natural = payload
        .get("natural_answer")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let context = payload
        .get("conversational_context")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let combined = [natural, context]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if combined.is_empty() {
        return None;
    }
    if grounding::leaks_pkc_bookkeeping(&combined) {
        return None;
    }
    Some(combined)
}

fn invoke_bridge(cfg: &ProviderConfig, question: &str) -> Option<String> {
    let script = Path::new(&cfg.pkc_bridge_script);
    if !script.is_absolute() || !script.is_file() {
        return None;
    }
    let python = if cfg.pkc_python_executable.trim().is_empty() {
        "python"
    } else {
        cfg.pkc_python_executable.trim()
    };
    let request = json!({
        "bridge_version": BRIDGE_VERSION,
        "request_id": "SAMMY-PKC-READONLY-001",
        "operation": "protected_retrieval",
        "question": question,
        "chapter_context": "Personal question — Sammy read-only PKC consult",
        "consumer_application": CONSUMER_APPLICATION,
        "recipient_class": "owner_dave",
        "realm": "private_autobiographical_interview",
        "purpose": "personal_consigliere",
        "disclosure_mode": "private_local",
        "requested_content_level": "full_text",
        "consent_state": "standing_authorization",
        "source_id": cfg.pkc_source_id.trim(),
    });

    let mut cmd = Command::new(python);
    cmd.arg(script)
        .arg("--timeout-seconds")
        .arg("20")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = script.parent() {
        cmd.current_dir(dir);
    }
    if !cfg.pkc_root.trim().is_empty() {
        cmd.arg("--pkc-root").arg(cfg.pkc_root.trim());
    }

    let mut child = cmd.spawn().ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(request.to_string().as_bytes()).ok()?;
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authorized_json(answer: &str) -> String {
        json!({
            "bridge_version": "1.0.0",
            "ok": true,
            "available": true,
            "payload": {
                "natural_answer": answer,
                "conversational_context": "",
                "authorized": true,
                "canonical_hash_verified": true,
                "protected_content_materialized": true
            }
        })
        .to_string()
    }

    #[test]
    fn authorized_payload_reaches_the_model_path() {
        let payload = conversational_payload_from_bridge_json(&authorized_json(
            "Dave grew up in Gloucester Township.",
        ));
        assert_eq!(
            payload.as_deref(),
            Some("Dave grew up in Gloucester Township.")
        );
    }

    #[test]
    fn unauthorized_payload_is_dropped() {
        let raw = json!({
            "bridge_version": "1.0.0",
            "ok": true,
            "available": true,
            "payload": {
                "natural_answer": "MUST_NOT_REACH_MODEL",
                "conversational_context": "",
                "authorized": false,
                "canonical_hash_verified": false,
                "protected_content_materialized": false,
                "safe_reason": "authorization_denied"
            }
        })
        .to_string();
        assert_eq!(conversational_payload_from_bridge_json(&raw), None);
    }

    #[test]
    fn unavailable_payload_is_dropped() {
        let raw = json!({
            "bridge_version": "1.0.0",
            "ok": false,
            "available": false,
            "payload": {
                "natural_answer": "",
                "authorized": false,
                "canonical_hash_verified": false,
                "protected_content_materialized": false,
                "safe_reason": "pkc_unavailable"
            }
        })
        .to_string();
        assert_eq!(conversational_payload_from_bridge_json(&raw), None);
    }

    #[test]
    fn bookkeeping_in_natural_answer_is_dropped() {
        assert_eq!(
            conversational_payload_from_bridge_json(&authorized_json(
                "authorization_denied CSDP-99"
            )),
            None
        );
    }

    #[test]
    fn disabled_config_does_not_query() {
        let cfg = ProviderConfig::default();
        assert!(!cfg.pkc_enabled);
        assert_eq!(authorized_payload_for_turn(&cfg, "Where did I grow up?"), None);
    }

    #[test]
    fn live_sammy_bridge_retrieves_authorized_seven_when_present() {
        let script = Path::new(r"D:\dev\StoryKeeper\tools\storykeeper_pkc_bridge.py");
        let root =
            Path::new(r"F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus");
        if !script.is_file() || !root.is_dir() {
            return;
        }
        let cfg = ProviderConfig {
            pkc_enabled: true,
            pkc_python_executable: "python".into(),
            pkc_bridge_script: script.to_string_lossy().into(),
            pkc_root: root.to_string_lossy().into(),
            pkc_source_id:
                "SRC-SHA256-9c53f4f121815dfd736e0377fafade3e649b2571f8858eebc8f1a92db2b0ec7f"
                    .into(),
            ..ProviderConfig::default()
        };
        let payload = authorized_payload_for_turn(
            &cfg,
            "What do you remember about moving as a child?",
        );
        let text = payload.expect("authorized Sammy retrieval should return a payload");
        assert!(!crate::grounding::leaks_pkc_bookkeeping(&text));
        assert!(text.len() > 20);
        assert!(!text.contains("CSDP-"));
        assert!(!text.contains("authorization_denied"));
        assert!(!text.contains("evidence_bookkeeping"));
    }
}
