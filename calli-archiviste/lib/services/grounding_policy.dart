/// Distinguishes source-backed facts, user testimony, and model inference.
///
/// A model follow-up may stay conversational, but it must not present concrete
/// sensory details, events, locations, objects, or memories as established
/// facts unless they appear in retrieved evidence or the user's statement.
class GroundingPolicy {
  static const promptBlock = '''
GROUNDING RULES (mandatory):
Distinguish three classes of material:
1. SOURCE-BACKED FACTS: only what appears in authorized retrieved evidence.
2. USER TESTIMONY: only what the person said in this conversation.
3. MODEL INFERENCE: conversational wording, empathy, and questions. Inference is not a remembered fact.

Do not introduce concrete sensory details, events, locations, objects, or memories as though they are established facts unless they appear in retrieved evidence or the user's current or prior statement in this conversation.
Do not invent a doorway, lighting, smell, weather, room, street, object, or episode to make a follow-up feel vivid.
You may ask open questions that invite the person to supply those details.
Never present inference as something you already know about their life.
Never mention PKC, retrieval, authorization, hashes, source IDs, or other bookkeeping.
''';

  static const List<String> unsupportedSceneMarkers = [
    'narrow doorway',
    'light shifted',
    'dim amber glow',
    'hallway turned left into the parlor',
    'it was raining that afternoon',
    'you were standing by the stove',
    "tightness around your father's jaw",
    'the screen door slammed',
    'the smell of oil heat',
    'first you unpacked, then you sat on the stairs',
  ];

  static const List<String> bookkeepingMarkers = [
    'CSDP-',
    'RULE-STORYKEEPER-PRIVATE-AUTOBIOGRAPHY',
    'evidence_bookkeeping',
    'record_trust_state',
    'authorization_denied',
    'canonical_hash',
    'protected_content_materialized',
  ];

  /// True when [modelText] treats a concrete scene as remembered fact that is
  /// absent from both evidence and the user's statement.
  static bool presentsUnsupportedConcreteScene({
    required String modelText,
    required String evidenceText,
    required String userText,
  }) {
    final allowed = '${evidenceText.toLowerCase()}\n${userText.toLowerCase()}';
    final hay = modelText.toLowerCase();
    for (final marker in unsupportedSceneMarkers) {
      if (hay.contains(marker) && !allowed.contains(marker)) {
        return true;
      }
    }
    return false;
  }

  static bool leaksPkcBookkeeping(String text) {
    return bookkeepingMarkers.any(text.contains);
  }
}