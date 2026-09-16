/// Stage 5-b-1 test plan coverage (design §8, items 1/2/3/6/7).
///
/// Runs on the Bench dev machine: `flutter test` inside calli-archiviste/.
/// The engine is pure Dart, so these tests are deterministic: injected
/// id factory and clock, static bank wording, no model, no I/O.
library;

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  var tick = 0;
  late DefaultProbeChainEngine engine;

  setUp(() {
    tick = 0;
    engine = DefaultProbeChainEngine(
      idFactory: () => 'test-${++tick}',
      clock: () => DateTime.utc(2026, 9, 16, 12, 0, tick),
    );
  });

  ProbeChainState fullChain() {
    final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
    engine.next(s, 'We went to the lake every summer.');            // surface
    engine.next(s, '  Pine needles and outboard oil, hot air.  ');  // sensory
    engine.next(s, 'Firsthand — I was in the boat.');               // source
    engine.next(s, 'A photo from that trip says June, not July.');  // contrast
    engine.next(s, "It's why water still means summer to me.");     // meaning
    return s;
  }

  group('I-4: the engine cannot approve', () {
    test('mint returns candidates only, never approved/closed', () {
      final s = fullChain();
      final candidates = engine.mint(s);
      expect(candidates, isNotEmpty);
      for (final c in candidates) {
        expect(c.status, RecordStatus.candidate,
            reason: 'mint() must never produce ${c.status}');
        expect(c.status, isNot(RecordStatus.approved));
        expect(c.status, isNot(RecordStatus.closed));
        expect(c.isCandidateOnly, isTrue);
      }
    });

    test('gate rejects every non-dave actor, including the engine', () {
      for (final actor in ['engine', 'sam', 'calli', 'model', 'script', '']) {
        final d = GatePolicy.evaluate(
          actor: actor,
          action: GateAction.approve,
          currentStatus: RecordStatus.candidate,
        );
        expect(d.allowed, isFalse, reason: 'actor "$actor" must be denied');
        expect(d.reason, contains('dave'));
      }
      // The one actor that passes.
      final dave = GatePolicy.evaluate(
        actor: GatePolicy.approvedActor,
        action: GateAction.approve,
        currentStatus: RecordStatus.candidate,
      );
      expect(dave.allowed, isTrue);
    });

    test('gate enforces legal transitions, not just the actor', () {
      // close requires approved (I-2).
      final closeCandidate = GatePolicy.evaluate(
        actor: 'dave',
        action: GateAction.close,
        currentStatus: RecordStatus.candidate,
      );
      expect(closeCandidate.allowed, isFalse);

      // approve -> close is legal.
      final closeApproved = GatePolicy.evaluate(
        actor: 'dave',
        action: GateAction.close,
        currentStatus: RecordStatus.approved,
      );
      expect(closeApproved.allowed, isTrue);
    });
  });

  group('I-2 / I-5: closure rule', () {
    test('chain below contrast (L4) refuses self-closure', () {
      final s = engine.start(mode: ElicitationMode.photo, topic: 'the lake');
      engine.next(s, 'A photo of the lake at dawn.');   // surface
      engine.next(s, 'Fog on the water, cold hands.');  // sensory
      final report = engine.closureStatus(s);
      expect(report.reachedContrast, isFalse);
      expect(report.selfCloseEligible, isFalse);
      expect(report.reason, contains('I-2'));
    });

    test('skip at L2-L4 poisons closure even when the chain reaches L5', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(s, 'We went every summer.');               // surface
      engine.skip(s, "I don't remember the details.");        // sensory SKIP
      engine.next(s, 'I was told that by my mother.');        // source
      engine.next(s, 'The photo says June, not July.');       // contrast
      engine.next(s, 'Water still means summer to me.');      // meaning
      final report = engine.closureStatus(s);
      expect(report.reachedContrast, isTrue);
      expect(report.poisonedBySkip, isTrue);
      expect(report.selfCloseEligible, isFalse);
      expect(report.reason, contains('I-5'));
    });

    test('contrast anchored with no poisoned skips is closable', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(s, 'We went every summer.');
      engine.next(s, 'Pine needles and outboard oil.');
      engine.next(s, 'Firsthand — I was in the boat.');
      engine.next(s, 'A photo says June, not July.');
      // Stop before meaning on purpose: contrast is enough.
      final report = engine.closureStatus(s);
      expect(report.selfCloseEligible, isTrue);
      expect(report.poisonedBySkip, isFalse);
    });
  });

  group('I-1: escalation gate', () {
    test('blank answer does not escalate; re-ask stays at the level', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      final first = s.pendingQuestion;
      expect(first, isNotEmpty);

      final r1 = engine.next(s, '   ');
      expect(r1.escalated, isFalse);
      expect(s.currentLevel, ProbeLevel.surface);
      expect(s.turns, isEmpty, reason: 'blank answers are not turns');
      expect(s.pendingQuestion, isNot(equals(first)),
          reason: 're-ask rotates wording');

      final r2 = engine.next(s, 'We went every summer.');
      expect(r2.escalated, isTrue);
      expect(s.currentLevel, ProbeLevel.sensory);
    });

    test('explicit skip escalates and is recorded verbatim in the audit', () {
      final s = engine.start(mode: ElicitationMode.audio, topic: 'the tape');
      engine.next(s, 'The tape is my grandfather.');
      engine.skip(s, "I don't remember");
      expect(s.skips.single.level, ProbeLevel.sensory);
      expect(s.skips.single.reason, "I don't remember");
      expect(s.currentLevel, ProbeLevel.source);
    });
  });

  group('I-3: verbatim fidelity (golden fixture)', () {
    test('full L1->L5 chain mints event @1.0 + judgment @1.0, byte-exact', () {
      final s = fullChain();
      final candidates = engine.mint(s);

      expect(candidates, hasLength(2));
      final event = candidates[0];
      final judgment = candidates[1];

      expect(event.recordType, RecordType.event);
      expect(event.confidence, RecordCandidate.eventConfidence);
      expect(judgment.recordType, RecordType.judgment);
      expect(judgment.confidence, RecordCandidate.judgmentConfidenceFull);

      // I-3: the canonical text is the surface answer EXACTLY as given —
      // the engine never trims or normalizes.
      expect(event.canonicalText, 'We went to the lake every summer.');

      // The sensory answer had leading/trailing spaces on purpose.
      expect(
        s.verbatimAt(ProbeLevel.sensory),
        '  Pine needles and outboard oil, hot air.  ',
      );

      // The chain walks all five levels in order.
      expect(event.probeChain, [
        'surface', 'sensory', 'source', 'contrast', 'meaning',
      ]);
      expect(event.sources, ['@session:${s.sessionId}']);
      expect(s.isComplete, isTrue);
    });

    test('chain stopped at contrast mints judgment @0.8', () {
      final s = engine.start(mode: ElicitationMode.document, topic: 'the letter');
      engine.next(s, 'A letter from 1987.');
      engine.next(s, 'Faded ink, creased twice.');
      engine.next(s, 'I found it myself in the box.');
      engine.next(s, 'The date contradicts the story I was told.');
      final candidates = engine.mint(s);
      expect(candidates, hasLength(2));
      expect(candidates[1].recordType, RecordType.judgment);
      expect(candidates[1].confidence, RecordCandidate.judgmentConfidenceContrast);
    });

    test('empty chain mints nothing', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'void');
      engine.next(s, '   ');
      expect(engine.mint(s), isEmpty);
    });

    test('L2-L4 skip downgrades to memory_candidate @0.3', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(s, 'We went every summer.');
      engine.skip(s, "I don't remember the details."); // sensory skip
      final candidates = engine.mint(s);
      expect(candidates, hasLength(2));
      expect(candidates[1].recordType, RecordType.memoryCandidate);
      expect(
        candidates[1].confidence,
        RecordCandidate.memoryCandidateConfidence,
      );
      expect(candidates[1].probeChain, contains('sensory.skip'));
    });
  });

  group('persona lock (mock model path)', () {
    test('in-character proposal is used verbatim', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(s, 'We went every summer.',
          modelProposal: 'What did the shore look like that first morning?');
      expect(s.pendingQuestionSource, QuestionSource.modelProposal);
      expect(s.pendingQuestion,
          'What did the shore look like that first morning?');
    });

    test('break-character proposal is discarded; static bank stands', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(s, 'We went every summer.',
          modelProposal:
              'As an AI language model, I cannot recall sensory details.');
      expect(s.pendingQuestionSource, QuestionSource.staticBank);
      expect(s.personaDiscards, hasLength(1));
      expect(s.personaDiscards.single.reason, contains('break-character'));
      // The fallback question is still a valid sensory question.
      expect(s.pendingQuestion, isNotEmpty);
      expect(s.currentLevel, ProbeLevel.sensory);
    });

    test('discard -> retry once -> valid retry is used', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(
        s,
        'We went every summer.',
        modelProposal: 'I am an AI and cannot feel pine needles.',
        retryProposal: 'What do you still smell when you think of the lake?',
      );
      expect(s.personaDiscards, hasLength(1));
      expect(s.pendingQuestionSource, QuestionSource.retryProposal);
      expect(s.pendingQuestion,
          'What do you still smell when you think of the lake?');
    });

    test('discard -> invalid retry -> bank fallback, audit keeps both', () {
      final s = engine.start(mode: ElicitationMode.freeRecall, topic: 'the lake');
      engine.next(
        s,
        'We went every summer.',
        modelProposal: 'Sorry, I cannot help with that.',
        retryProposal: 'My knowledge cutoff prevents me from knowing lakes.',
      );
      expect(s.personaDiscards, hasLength(2));
      expect(s.pendingQuestionSource, QuestionSource.staticBank);
    });

    test('start() honors the same wording path for the surface level', () {
      final s = engine.start(
        mode: ElicitationMode.chapter,
        topic: 'the move',
        modelProposal: 'Tell me about the move — where does it start?',
      );
      expect(s.pendingQuestionSource, QuestionSource.modelProposal);
      expect(s.pendingQuestion, 'Tell me about the move — where does it start?');
    });
  });

  group('round-trip', () {
    test('state serializes and restores losslessly', () {
      final s = fullChain();
      final restored = ProbeChainState.fromJson(
        Map<String, dynamic>.from(s.toJson() as Map),
      );
      expect(restored.sessionId, s.sessionId);
      expect(restored.topic, s.topic);
      expect(restored.currentLevel, s.currentLevel);
      expect(restored.turns.map((t) => t.answerVerbatim).toList(),
          s.turns.map((t) => t.answerVerbatim).toList());
      expect(restored.pendingQuestion, s.pendingQuestion);
      expect(engine.mint(restored).length, engine.mint(s).length);
    });
  });
}
