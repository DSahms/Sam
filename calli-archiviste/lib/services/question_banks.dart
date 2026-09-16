/// Offline static question banks — Stage 5-b-1.
///
/// The model owns wording at runtime; this bank owns the FLOOR. When the
/// model is offline (Kobold down, M40 busy), when the persona lock discards
/// a model proposal twice (break-character -> discard -> retry once ->
/// static fallback), or when a test needs a deterministic fixture, these
/// questions are what the chain walks with.
///
/// The pattern is the Mother Leeds chain (the golden standard): five
/// levels, each one pressing one layer deeper, never skipping ahead, never
/// padding with pleasantries. Variants are Leeds-pattern reconstructions:
/// same shape (surface -> sensory pin -> source -> contrast -> meaning),
/// topic-slot substitutable.
library;

import 'package:calli_archiviste/services/probe_chain_engine.dart';

/// Rotating, topic-aware static question bank. Pure Dart, deterministic:
/// the same (level, topic, turnIndex) always yields the same question.
class QuestionBank {
  const QuestionBank();

  /// The Mother Leeds five-level bank. One slot per probe level, ordered
  /// variants; variant 0 is the canonical form for that level.
  static const Map<ProbeLevel, List<String>> motherLeeds = {
    ProbeLevel.surface: [
      'Tell me about {topic} — start wherever it starts for you.',
      'Walk me through {topic} from the beginning. What happened?',
      'If {topic} were a chapter, what would its first line be?',
      'What is the story of {topic}, told your way?',
    ],
    ProbeLevel.sensory: [
      'Pin one concrete detail from that moment — what did you actually see or hear?',
      'Where were you standing, and what was the first thing your body noticed?',
      'What did {topic} sound like? Smell like? Pick one and stay with it.',
      'Close your eyes: what is the single sharpest sensory detail you still have?',
    ],
    ProbeLevel.source: [
      'How do you know that? Did you see it yourself, or were you told?',
      'Where does that detail come from — your own eyes, someone else\'s account, or something you read?',
      'Who else was there, and would they tell it the same way?',
      'Is that memory firsthand, or has it been handed to you? By whom?',
    ],
    ProbeLevel.contrast: [
      'Has anything ever contradicted that version — a record, a photo, another person?',
      'You have said other things about {topic} that do not quite line up. How do they fit together?',
      'What would someone who disagrees with your account point at first?',
      'If a record surfaced tomorrow that contradicted this, which detail would it attack?',
    ],
    ProbeLevel.meaning: [
      'Why does {topic} matter? What would be lost if this memory were lost?',
      'What did that change about how you moved through the world afterward?',
      'When you weigh everything, what is the meaning of {topic} for you?',
      'If Calli could keep only one line from this, what should it be?',
    ],
  };

  /// Template used when an answer is blank: the chain stays at the current
  /// level (invariant I-1 — no escalation without substance or skip) and
  /// re-asks in a quieter register instead of pushing deeper.
  static const Map<ProbeLevel, List<String>> reasks = {
    ProbeLevel.surface: [
      'Take your time. Even one sentence about {topic} is a start.',
      'Nothing has to be polished. What comes to mind first about {topic}?',
    ],
    ProbeLevel.sensory: [
      'Maybe start smaller — one color, one sound, one object in the room.',
      'If the big picture is blurry, what fragment is still sharp?',
    ],
    ProbeLevel.source: [
      'No pressure to be certain — who would know more about this besides you?',
      'Even a guess at the source helps: firsthand, told to you, or read somewhere?',
    ],
    ProbeLevel.contrast: [
      'Does any part of that version feel shaky when you say it out loud?',
      'Has anyone ever remembered {topic} differently?',
    ],
    ProbeLevel.meaning: [
      'Why do you think this one stayed with you?',
      'What would you want someone in ten years to understand about {topic}?',
    ],
  };

  /// Deterministic lookup with {topic} substitution. [turnIndex] rotates
  /// through variants so a re-asked level does not repeat itself verbatim;
  /// negative or oversized indexes wrap safely.
  String questionFor(
    ProbeLevel level, {
    required String topic,
    required int turnIndex,
    bool isReask = false,
  }) {
    final bank = isReask ? reasks[level]! : motherLeeds[level]!;
    final normalized = turnIndex < 0 ? 0 : turnIndex;
    final template = bank[normalized % bank.length];
    return template.replaceAll('{topic}', topic);
  }
}
