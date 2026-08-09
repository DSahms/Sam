//! `citations` — citation and trust interface (Phase 5).
//!
//! Directive §25. Citation precision preserves the most specific available
//! location. Every source-grounded answer provides citations, and every answer
//! has an overall trust classification. Citation IDs always resolve to real
//! records; we never fabricate IDs.

use serde::{Deserialize, Serialize};

/// The most specific location of a cited piece of content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CitationLocation {
    /// A knowledge record by id.
    KnowledgeRecord { record_id: String },
    /// A source by id, optionally with a text location hint.
    Source {
        source_id: String,
        detail: Option<String>,
    },
    /// A conversation message.
    Conversation {
        conversation_id: String,
        message_id: String,
    },
    /// A structured import row/field.
    StructuredRow {
        source_id: String,
        row: usize,
        field: Option<String>,
    },
}

/// A single citation: what was cited and where it lives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Stable id of this citation within an answer. Always references a real
    /// record/source id (never fabricated — directive §25).
    pub id: String,
    pub location: CitationLocation,
    /// The text snippet that grounds the claim.
    pub snippet: String,
}

/// Overall trust classification for a grounded answer (directive §25).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustClassification {
    /// Fully supported by approved sources.
    SourceSupported,
    /// Mixes source-grounded content and inference.
    MixedSourceAndInference,
    /// Pure inference, no source grounding.
    Inference,
    /// A suggestion, not a claim.
    Suggestion,
    /// Evidence insufficient.
    Unknown,
}

impl TrustClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            TrustClassification::SourceSupported => "source_supported",
            TrustClassification::MixedSourceAndInference => "mixed_source_and_inference",
            TrustClassification::Inference => "inference",
            TrustClassification::Suggestion => "suggestion",
            TrustClassification::Unknown => "unknown",
        }
    }
}

/// A per-claim label when an answer mixes knowledge types (directive §25).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimLabel {
    StoredFact,
    SourcedClaim,
    ApprovedConclusion,
    Inference,
    Suggestion,
    Uncertain,
}

impl ClaimLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            ClaimLabel::StoredFact => "stored_fact",
            ClaimLabel::SourcedClaim => "sourced_claim",
            ClaimLabel::ApprovedConclusion => "approved_conclusion",
            ClaimLabel::Inference => "inference",
            ClaimLabel::Suggestion => "suggestion",
            ClaimLabel::Uncertain => "uncertain",
        }
    }
}

/// A material factual claim block within an answer, with its label and the
/// citations that ground it (if any).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimBlock {
    pub text: String,
    pub label: ClaimLabel,
    /// Citation ids within this answer.
    pub citations: Vec<String>,
}

/// A fully-annotated answer with citations, trust, and labeled claim blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedAnswer {
    pub plain_text: String,
    pub trust: TrustClassification,
    pub citations: Vec<Citation>,
    pub claims: Vec<ClaimBlock>,
}

impl AnnotatedAnswer {
    /// An answer with no source grounding (pure inference).
    pub fn inference(text: impl Into<String>) -> Self {
        Self {
            plain_text: text.into(),
            trust: TrustClassification::Inference,
            citations: vec![],
            claims: vec![],
        }
    }

    /// An "I don't know" answer (directive §25: state that Sammy does not know
    /// when evidence is insufficient).
    pub fn unknown() -> Self {
        Self {
            plain_text: "I don't know.".into(),
            trust: TrustClassification::Unknown,
            citations: vec![],
            claims: vec![],
        }
    }

    /// Validate that every citation id referenced in claim blocks exists in the
    /// answer's citations list. Returns false if any fabricated id is found
    /// (directive §25: never generate a citation id that does not exist).
    pub fn citations_are_consistent(&self) -> bool {
        let known: std::collections::HashSet<&str> =
            self.citations.iter().map(|c| c.id.as_str()).collect();
        for claim in &self.claims {
            for cid in &claim.citations {
                if !known.contains(cid.as_str()) {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_classification_serializes() {
        let t = TrustClassification::SourceSupported;
        let s = serde_json::to_string(&t).unwrap();
        assert!(s.contains("source_supported"));
    }

    #[test]
    fn inference_answer_has_no_citations() {
        let a = AnnotatedAnswer::inference("a guess");
        assert_eq!(a.trust, TrustClassification::Inference);
        assert!(a.citations.is_empty());
    }

    #[test]
    fn unknown_answer_is_honest() {
        let a = AnnotatedAnswer::unknown();
        assert_eq!(a.trust, TrustClassification::Unknown);
        assert!(a.plain_text.to_lowercase().contains("don't know"));
    }

    #[test]
    fn citations_consistency_detects_fabricated_id() {
        let a = AnnotatedAnswer {
            plain_text: "x".into(),
            trust: TrustClassification::SourceSupported,
            citations: vec![Citation {
                id: "c1".into(),
                location: CitationLocation::KnowledgeRecord {
                    record_id: "r1".into(),
                },
                snippet: "...".into(),
            }],
            claims: vec![ClaimBlock {
                text: "claim".into(),
                label: ClaimLabel::StoredFact,
                citations: vec!["c1".into()],
            }],
        };
        assert!(a.citations_are_consistent());

        let bad = AnnotatedAnswer {
            plain_text: "x".into(),
            trust: TrustClassification::SourceSupported,
            citations: vec![],
            claims: vec![ClaimBlock {
                text: "claim".into(),
                label: ClaimLabel::StoredFact,
                citations: vec!["c1".into()],
            }],
        };
        assert!(!bad.citations_are_consistent());
    }

    #[test]
    fn claim_labels_round_trip() {
        for l in [
            ClaimLabel::StoredFact,
            ClaimLabel::SourcedClaim,
            ClaimLabel::ApprovedConclusion,
            ClaimLabel::Inference,
            ClaimLabel::Suggestion,
            ClaimLabel::Uncertain,
        ] {
            let s = serde_json::to_string(&l).unwrap();
            let back: ClaimLabel = serde_json::from_str(&s).unwrap();
            assert_eq!(l, back);
        }
    }
}
