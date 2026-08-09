//! `tools` — agentic tools (Phase 8).
//!
//! Directive §28. The first release includes only:
//! - Read-only local source search
//! - Read-only corpus lookup
//! - Draft generation (the owner reviews; nothing is sent)
//! - Planning that does not execute external actions
//! - Mock tools for permission testing
//!
//! NO external actions execute (no email, posting, purchases, file deletion,
//! shell, browser automation, etc.). The permission architecture
//! ([`crate::permissions`]) gates every future tool.

use crate::permissions::{Reversibility, RiskLevel, ToolDeclaration};

/// The registry of tools available in this release. Each entry is a static
/// declaration the runtime uses to build action previews for confirmation.
pub fn registry() -> Vec<ToolDeclaration> {
    vec![
        ToolDeclaration {
            tool_id: "source_search".into(),
            operation: "Search local sources by keyword".into(),
            data_accessed: "Encrypted source extracted text (in-vault)".into(),
            data_leaving_device: "None".into(),
            destination: "Local only".into(),
            risk: RiskLevel::ReadOnly,
            reversibility: Reversibility::Reversible,
        },
        ToolDeclaration {
            tool_id: "corpus_lookup".into(),
            operation: "Look up an approved knowledge record by id".into(),
            data_accessed: "Knowledge records (in-vault)".into(),
            data_leaving_device: "None".into(),
            destination: "Local only".into(),
            risk: RiskLevel::ReadOnly,
            reversibility: Reversibility::Reversible,
        },
        ToolDeclaration {
            tool_id: "draft_generation".into(),
            operation: "Generate a draft for owner review".into(),
            data_accessed: "Conversation context + retrieved knowledge".into(),
            data_leaving_device: "None (draft only)".into(),
            destination: "Local only".into(),
            risk: RiskLevel::DraftOnly,
            reversibility: Reversibility::Reversible,
        },
        ToolDeclaration {
            tool_id: "planning".into(),
            operation: "Produce a non-executing plan".into(),
            data_accessed: "Conversation context".into(),
            data_leaving_device: "None".into(),
            destination: "Local only".into(),
            risk: RiskLevel::ReadOnly,
            reversibility: Reversibility::Reversible,
        },
        ToolDeclaration {
            tool_id: "mock_external".into(),
            operation: "Mock tool for permission testing (never executes)".into(),
            data_accessed: "None".into(),
            data_leaving_device: "None (mock)".into(),
            destination: "None (mock)".into(),
            risk: RiskLevel::MutatesExternal,
            reversibility: Reversibility::Irreversible,
        },
    ]
}

/// Find a tool declaration by id.
pub fn find(tool_id: &str) -> Option<ToolDeclaration> {
    registry().into_iter().find(|t| t.tool_id == tool_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_five_tools() {
        let r = registry();
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn all_tools_declare_no_real_external_action() {
        for t in registry() {
            // Every tool must either be read-only or draft-only or a mock;
            // none execute a real external action in this release.
            assert!(
                matches!(
                    t.risk,
                    RiskLevel::ReadOnly
                        | RiskLevel::DraftOnly
                        | RiskLevel::MutatesExternal
                ),
                "{} has unexpected risk",
                t.tool_id
            );
            // The mock is the only MutatesExternal, and it is explicitly a mock.
            if t.risk == RiskLevel::MutatesExternal {
                assert!(
                    t.data_leaving_device.contains("mock")
                        || t.operation.contains("Mock")
                );
            }
        }
    }

    #[test]
    fn find_returns_declaration() {
        let t = find("source_search").unwrap();
        assert_eq!(t.tool_id, "source_search");
        assert!(find("nonexistent").is_none());
    }
}
