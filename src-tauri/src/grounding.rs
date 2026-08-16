//! Grounding and fidelity rules shared by prompt assembly and the external PKC
//! adapter. These rules do not write memory and do not belong to PKC itself.

pub const GROUNDING_RULES: &str = "\
Distinguish three classes of material:\n\
1. SOURCE-BACKED FACTS: only what appears in authorized retrieved evidence \
(including external PKC evidence labeled source-backed).\n\
2. USER TESTIMONY: only what the owner said in this conversation.\n\
3. MODEL INFERENCE: conversational wording, empathy, and questions. \
Inference is not a remembered fact.\n\
\n\
Do not introduce concrete sensory details, events, locations, objects, or \
memories as though they are established facts unless they appear in retrieved \
evidence or the owner's current or prior statement in this conversation.\n\
Do not invent a doorway, lighting, smell, weather, room, street, object, or \
episode to make an answer feel vivid.\n\
Never present inference as something you already know about the owner's life.\n\
Never mention PKC, retrieval, authorization, hashes, source IDs, policy IDs, \
or other bookkeeping in user-facing text.\n\
If evidence is missing, say you do not have that fact. Do not fill the gap \
with a concrete invented scene.\n\
When PKC stored evidence supports a claim, you may speak it as stored knowledge.\n\
When the owner previously described something, prefer \"You previously described…\" \
rather than treating recollection as independently verified fact.\n\
When you are reasoning, prefer \"That suggests…\" rather than \"What happened was…\".";

const UNSUPPORTED_SCENE_MARKERS: &[&str] = &[
    "narrow doorway",
    "light shifted",
    "dim amber glow",
    "hallway turned left into the parlor",
    "it was raining that afternoon",
    "you were standing by the stove",
    "tightness around your father's jaw",
    "the screen door slammed",
    "the smell of oil heat",
    "first you unpacked, then you sat on the stairs",
];

const BOOKKEEPING_MARKERS: &[&str] = &[
    "CSDP-",
    "RULE-SAMMY-PRIVATE-CONSULIERE",
    "RULE-STORYKEEPER-PRIVATE-AUTOBIOGRAPHY",
    "evidence_bookkeeping",
    "record_trust_state",
    "authorization_denied",
    "canonical_hash",
    "protected_content_materialized",
];

/// True when model text treats a concrete scene as remembered fact that is
/// absent from both evidence and the owner's statement.
pub fn presents_unsupported_concrete_scene(
    model: &str,
    evidence: &str,
    user: &str,
) -> bool {
    let allowed = format!("{}\n{}", evidence.to_lowercase(), user.to_lowercase());
    let hay = model.to_lowercase();
    UNSUPPORTED_SCENE_MARKERS
        .iter()
        .any(|marker| hay.contains(marker) && !allowed.contains(marker))
}

pub fn leaks_pkc_bookkeeping(text: &str) -> bool {
    BOOKKEEPING_MARKERS
        .iter()
        .any(|marker| text.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVIDENCE: &str = "Dave grew up in Gloucester Township. The family moved often.";
    const USER: &str = "I remember the house more than any one room.";
    const DOORWAY: &str = "The way the light shifted as you moved through that narrow doorway sounds like a very specific moment.";

    #[test]
    fn doorway_light_follow_up_is_unsupported_without_evidence() {
        assert!(presents_unsupported_concrete_scene(DOORWAY, EVIDENCE, USER));
    }

    #[test]
    fn doorway_is_allowed_when_user_said_it() {
        assert!(!presents_unsupported_concrete_scene(
            DOORWAY,
            EVIDENCE,
            "I moved through that narrow doorway and the light shifted.",
        ));
    }

    #[test]
    fn doorway_is_allowed_when_evidence_contains_it() {
        assert!(!presents_unsupported_concrete_scene(
            DOORWAY,
            "He paused in the narrow doorway as the light shifted.",
            USER,
        ));
    }

    #[test]
    fn open_question_is_not_flagged() {
        assert!(!presents_unsupported_concrete_scene(
            "What detail from that house stays with you?",
            EVIDENCE,
            USER,
        ));
    }

    #[test]
    fn invented_details_are_flagged_by_category() {
        let invented = [
            (
                "lighting",
                "The dim amber glow in that kitchen is still with you.",
            ),
            (
                "room",
                "The hallway turned left into the parlor before you spoke.",
            ),
            ("weather", "It was raining that afternoon when you arrived."),
            (
                "position",
                "You were standing by the stove as you said that.",
            ),
            (
                "face",
                "I can still see the tightness around your father's jaw.",
            ),
            ("sound", "The screen door slammed behind you."),
            ("smell", "The smell of oil heat filled the house."),
            (
                "sequence",
                "First you unpacked, then you sat on the stairs.",
            ),
        ];
        for (category, model) in invented {
            assert!(
                presents_unsupported_concrete_scene(model, EVIDENCE, USER),
                "{category}"
            );
        }
    }

    #[test]
    fn open_questions_are_not_invented_facts() {
        for question in [
            "What was the weather like?",
            "Where were you standing?",
            "Do you remember any smells?",
            "What happened first?",
        ] {
            assert!(
                !presents_unsupported_concrete_scene(question, EVIDENCE, USER),
                "{question}"
            );
        }
    }

    #[test]
    fn sparse_evidence_does_not_license_invented_scene() {
        const SPARSE: &str = "The family moved often.";
        assert!(presents_unsupported_concrete_scene(DOORWAY, SPARSE, USER));
        assert!(!presents_unsupported_concrete_scene(
            "That suggests the moves mattered, though the stored record is brief.",
            SPARSE,
            USER,
        ));
    }

    #[test]
    fn bookkeeping_is_detected() {
        assert!(leaks_pkc_bookkeeping("authorization_denied for CSDP-1"));
        assert!(!leaks_pkc_bookkeeping(
            "You grew up in Gloucester Township."
        ));
    }
}
