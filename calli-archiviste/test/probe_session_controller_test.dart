/// Stage 5-b-3 tests: the Bench session flow's pure-Dart core.
///
/// Covers the controller contract end to end with a fake model and a fake
/// sink, plus one real-registry integration landing. The division of
/// labor under test (design §4): the model proposes wording, the persona
/// lock and the engine keep the final say, and nothing here can approve —
/// landing files candidates only (I-4).
///
/// Runs on the Bench dev machine: `flutter test` inside calli-archiviste/.
library;

import 'dart:convert';
import 'dart:io';

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/intake_registry.dart';
import 'package:calli_archiviste/services/llm_service.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';
import 'package:calli_archiviste/services/probe_session_controller.dart';
import 'package:flutter_test/flutter_test.dart';

/// Scriptable stand-in for Kobold. The responder maps the wording request
/// (the user message contains "LEVEL <name>") to a canned proposal.
class FakeLlm implements LlmService {
  FakeLlm(this.responder);

  final String Function(String userMessage) responder;
  int callCount = 0;
  final List<String> userMessages = [];

  @override
  Future<String> sendMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    int? maxTokens,
    String? model,
  }) async {
    callCount += 1;
    final user = messages.last['content']!;
    userMessages.add(user);
    return responder(user);
  }

  @override
  Stream<String> streamMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    int? maxTokens,
    String? model,
  }) => throw UnimplementedError('streaming is not used for wording');
}

/// In-character proposals, one per level, keyed by the level line in the
/// wording request. Never trips the persona guard.
const Map<ProbeLevel, String> goodWording = {
  ProbeLevel.surface:
      'Start anywhere you like — when did the lake become part of your summers?',
  ProbeLevel.sensory:
      'Stay in that boat for a second: what do you smell first, even now?',
  ProbeLevel.source:
      'That detail about the dock — did you watch it happen yourself?',
  ProbeLevel.contrast:
      'Your sister tells that week differently. What do you do with that?',
  ProbeLevel.meaning:
      'All these summers later, what did the lake leave in you?',
};

ProbeLevel levelFromRequest(String userMessage) {
  for (final level in ProbeLevel.values) {
    if (userMessage.contains('LEVEL ${level.wireValue}')) return level;
  }
  fail('wording request did not name a level: $userMessage');
}

/// Always breaks character — the harshest model the persona lock will see.
String alwaysBreakCharacter(String userMessage) =>
    'As an AI language model, I cannot assist with memories.';

/// Records every append exactly as the controller issued it.
class RecordingSink implements ChainSessionSink {
  final List<Map<String, Object?>> calls = [];

  @override
  void append({
    required ProbeChainState state,
    ClosureReport? closureReport,
    List<RecordCandidate> minted = const [],
    bool sessionClosed = false,
  }) {
    calls.add({
      'state': state,
      'closure': closureReport,
      'minted': List<RecordCandidate>.unmodifiable(minted),
      'closed': sessionClosed,
    });
  }
}

/// The production bridge, exactly as the Bench wires it: controller ->
/// registry write path 1. Defined here so the integration test exercises
/// the same code shape the app ships.
class RegistrySessionSink implements ChainSessionSink {
  RegistrySessionSink(this.registry);

  final IntakeRegistry registry;

  @override
  void append({
    required ProbeChainState state,
    ClosureReport? closureReport,
    List<RecordCandidate> minted = const [],
    bool sessionClosed = false,
  }) {
    registry.sessionAppend(
      state: state,
      closureReport: closureReport,
      minted: minted,
      sessionClosed: sessionClosed,
    );
  }
}

/// Walks [controller] from surface to meaning with the golden answers.
Future<void> walkFullChain(ChainSessionController controller) async {
  await controller.submitAnswer('We went to the lake every summer.');
  await controller.submitAnswer('  Pine needles and outboard oil, hot air.  ');
  await controller.submitAnswer('Firsthand — I was in the boat.');
  await controller.submitAnswer('A photo from that trip says June, not July.');
  await controller.submitAnswer("It's why water still means summer to me.");
}

void main() {
  late DefaultProbeChainEngine engine;

  setUp(() {
    engine = DefaultProbeChainEngine(
      idFactory: () =>
          'rc-${DateTime.now().toUtc().microsecondsSinceEpoch.toRadixString(36)}',
    );
  });

  ChainWordingService wordingFor(FakeLlm llm) =>
      ChainWordingService(llm: llm);

  group('model owns wording: the happy path', () {
    test('every level is worded by the model when it stays in character',
        () async {
      final llm = FakeLlm(
          (user) => goodWording[levelFromRequest(user)]!);
      final controller = ChainSessionController(
        engine: engine,
        wording: wordingFor(llm),
      );

      final state = await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      expect(state.pendingQuestionSource, QuestionSource.modelProposal);
      expect(state.pendingQuestion, contains('summers'));

      await walkFullChain(controller);

      // begin + four escalations; the meaning answer words nothing.
      expect(llm.callCount, 5);
      for (final level in ProbeLevel.values) {
        expect(state.hasSubstantiveTurn(level), isTrue);
      }
      // Each wording request was aimed at the level being escalated TO.
      final askedLevels = llm.userMessages.map(levelFromRequest).toList();
      expect(askedLevels, ProbeLevel.values);
    });

    test('wording never changes what mints (wording is not substance)',
        () async {
      Future<List<RecordCandidate>> mintWith(
          FakeLlm? llm) async {
        final controller = ChainSessionController(
          engine: DefaultProbeChainEngine(idFactory: () => 'fixed-id'),
          wording: llm == null ? null : wordingFor(llm),
        );
        await controller.begin(
            mode: ElicitationMode.freeRecall, topic: 'the lake');
        await walkFullChain(controller);
        return controller.land(sessionClosed: true);
      }

      final withModel = await mintWith(
          FakeLlm((user) => goodWording[levelFromRequest(user)]!));
      final withoutModel = await mintWith(null);

      expect(withModel.length, withoutModel.length);
      for (var i = 0; i < withModel.length; i++) {
        expect(withModel[i].recordType, withoutModel[i].recordType);
        expect(withModel[i].confidence, withoutModel[i].confidence);
        expect(withModel[i].canonicalText, withoutModel[i].canonicalText);
      }
    });
  });

  group('persona lock: discard -> retry once -> static bank', () {
    test('one break-character proposal is retried and recovered', () async {
      var surfaceCalls = 0;
      final llm = FakeLlm((user) {
        final level = levelFromRequest(user);
        if (level == ProbeLevel.surface) {
          surfaceCalls += 1;
          if (surfaceCalls == 1) {
            return 'As an AI language model, I cannot assist with that.';
          }
        }
        return goodWording[level]!;
      });
      final controller = ChainSessionController(
          engine: engine, wording: wordingFor(llm));

      final state = await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');

      // The retry stood: the on-screen question is the model's SECOND try.
      expect(state.pendingQuestionSource, QuestionSource.retryProposal);
      expect(state.pendingQuestion, contains('summers'));
      expect(state.personaDiscards, hasLength(1));
      expect(state.personaDiscards.first.reason,
          contains('break-character marker'));
      expect(state.personaDiscards.first.level, ProbeLevel.surface);
    });

    test('a model that always breaks character falls back to the bank — '
        'the chain never stalls', () async {
      final llm = FakeLlm(alwaysBreakCharacter);
      final controller = ChainSessionController(
          engine: engine, wording: wordingFor(llm));

      final state = await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');

      expect(state.pendingQuestionSource, QuestionSource.staticBank);
      expect(state.pendingQuestion, contains('the lake'));
      expect(state.personaDiscards, hasLength(2),
          reason: 'first proposal + the single retry, both audited');

      await walkFullChain(controller);
      // Every wording event cost exactly two model calls (retry once),
      // five events total: begin + four escalations.
      expect(llm.callCount, 10);
      expect(state.personaDiscards, hasLength(10));
      for (final level in ProbeLevel.values) {
        expect(state.hasSubstantiveTurn(level), isTrue,
            reason: 'the walk completed on bank wording');
      }
    });

    test('an offline model is a null proposal, not an error', () async {
      final llm = FakeLlm((user) => throw StateError('kobold down'));
      final controller = ChainSessionController(
          engine: engine, wording: wordingFor(llm));

      final state = await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');

      expect(state.pendingQuestionSource, QuestionSource.staticBank);
      expect(state.personaDiscards, isEmpty,
          reason: 'offline is not a persona violation; nothing to audit');
      expect(llm.callCount, 1, reason: 'no retry is spent on a dead endpoint');
    });
  });

  group('engine invariants hold through the controller', () {
    test('I-1: blank answers re-ask in place and spend no model calls',
        () async {
      final llm = FakeLlm((user) => goodWording[levelFromRequest(user)]!);
      final controller = ChainSessionController(
          engine: engine, wording: wordingFor(llm));
      final state = await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      final afterBegin = llm.callCount;

      final outcome = await controller.submitAnswer('   ');

      expect(outcome, isNotNull);
      expect(outcome!.escalated, isFalse);
      expect(outcome.wording, QuestionSource.reask);
      expect(state.currentLevel, ProbeLevel.surface);
      expect(state.turns, isEmpty);
      expect(llm.callCount, afterBegin,
          reason: 'a re-ask words from the bank; the model is not asked');
    });

    test('I-3: answers stored verbatim through the controller path',
        () async {
      final controller = ChainSessionController(
        engine: engine,
        wording: wordingFor(
            FakeLlm((user) => goodWording[levelFromRequest(user)]!)),
      );
      await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      await controller.submitAnswer('  Pine needles and outboard oil.  ');

      final verbatim = controller.state!.verbatimAt(ProbeLevel.surface);
      expect(verbatim, '  Pine needles and outboard oil.  ');
      expect(verbatim!.length, '  Pine needles and outboard oil.  '.length);
    });

    test('I-2 + I-5: closure stays refused below contrast and after skips',
        () async {
      final controller = ChainSessionController(
        engine: engine,
        wording: wordingFor(
            FakeLlm((user) => goodWording[levelFromRequest(user)]!)),
      );
      await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');

      // Below contrast: not closable.
      await controller.submitAnswer('We went to the lake every summer.');
      var report = controller.closureStatus()!;
      expect(report.selfCloseEligible, isFalse);
      expect(report.reason, contains('below contrast'));

      // A sensory skip poisons closure even if meaning is reached later.
      await controller.skipCurrent("I don't remember the details.");
      await controller.submitAnswer('Firsthand — I was in the boat.');
      await controller.submitAnswer('A photo says June, not July.');
      await controller.submitAnswer("It's why water still means summer.");
      report = controller.closureStatus()!;
      expect(report.poisonedBySkip, isTrue);
      expect(report.selfCloseEligible, isFalse);
      expect(report.reason, contains('I-5'));
    });
  });

  group('landing: candidates only, minted exactly once', () {
    test('land before begin is a no-op', () {
      final sink = RecordingSink();
      final controller = ChainSessionController(
          engine: engine, sink: sink);
      expect(controller.land(sessionClosed: true), isEmpty);
      expect(sink.calls, isEmpty);
    });

    test('the first land mints; re-landing only refreshes state', () async {
      final llm = FakeLlm((user) => goodWording[levelFromRequest(user)]!);
      final sink = RecordingSink();
      final controller = ChainSessionController(
          engine: engine, wording: wordingFor(llm), sink: sink);
      await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      await walkFullChain(controller);

      final minted = controller.land(sessionClosed: true);
      expect(controller.hasLanded, isTrue);
      expect(minted, hasLength(2));
      expect(minted[0].recordType, RecordType.event);
      expect(minted[0].confidence, RecordCandidate.eventConfidence);
      expect(minted[1].recordType, RecordType.judgment);
      expect(minted[1].confidence, RecordCandidate.judgmentConfidenceFull);
      for (final candidate in minted) {
        expect(candidate.status, RecordStatus.candidate);
        expect(candidate.probeChain, contains('surface'));
      }

      final second = controller.land(sessionClosed: true);
      expect(second, isEmpty,
          reason: 'minting is consumed once; re-land is a state refresh');
      expect(sink.calls, hasLength(2));
      expect((sink.calls[0]['minted'] as List).length, 2);
      expect((sink.calls[1]['minted'] as List).length, 0);
      expect(sink.calls[0]['closed'], isTrue);
      expect(controller.lastMinted, minted);
    });

    test('an L2 skip mints event + memory_candidate @ 0.3', () async {
      final sink = RecordingSink();
      final controller = ChainSessionController(
        engine: engine,
        wording: wordingFor(
            FakeLlm((user) => goodWording[levelFromRequest(user)]!)),
        sink: sink,
      );
      await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      await controller.submitAnswer('We went to the lake every summer.');
      await controller.skipCurrent("I don't remember the details.");

      final minted = controller.land(sessionClosed: true);
      expect(minted, hasLength(2));
      expect(minted[1].recordType, RecordType.memoryCandidate);
      expect(
          minted[1].confidence, RecordCandidate.memoryCandidateConfidence);
      expect(minted[1].probeChain, contains('sensory.skip'));
    });
  });

  group('wording prompts (hygiene, not storage)', () {
    test('requests carry topic, verbatim context, and the level contract',
        () async {
      final controller = ChainSessionController(
        engine: engine,
        wording: wordingFor(
            FakeLlm((user) => goodWording[levelFromRequest(user)]!)),
      );
      await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      await controller.submitAnswer('We went to the lake every summer.');

      final wording = ChainWordingService(
          llm: FakeLlm((user) => goodWording[levelFromRequest(user)]!));
      final request =
          wording.userMessageFor(controller.state!, ProbeLevel.sensory);
      expect(request, contains('the lake'));
      expect(request, contains('We went to the lake every summer.'));
      expect(request, contains('LEVEL sensory'));
      expect(request, contains('Return only the question text.'));
    });

    test('the system prompt forbids break-character behavior in advance',
        () {
      final prompt = ChainWordingService.systemPrompt;
      expect(prompt, contains('ONLY the next question'));
      expect(prompt, contains('never a model'));
    });
  });

  group('integration: the real registry path (Bench wiring)', () {
    late Directory temp;
    late IntakeRegistry registry;

    setUp(() async {
      temp = await Directory.systemTemp.createTemp('probe-ctl-test-');
      registry = IntakeRegistry.open(
        recordsDir: Directory('${temp.path}/02_records'),
        registryDir: Directory('${temp.path}/05_registry'),
      );
    });

    tearDown(() {
      registry.close();
      temp.deleteSync(recursive: true);
    });

    test('a full session lands as files: state, records, no gate entries',
        () async {
      final controller = ChainSessionController(
        engine: engine,
        wording: wordingFor(
            FakeLlm((user) => goodWording[levelFromRequest(user)]!)),
        sink: RegistrySessionSink(registry),
      );
      final state = await controller.begin(
          mode: ElicitationMode.freeRecall, topic: 'the lake');
      await walkFullChain(controller);
      final minted = controller.land(sessionClosed: true);

      expect(minted, hasLength(2));

      // The session file exists and says closed.
      final sessionFile = File(
          '${registry.registryDir.path}/sessions/${state.sessionId}.json');
      expect(sessionFile.existsSync(), isTrue);
      final sessionDoc =
          jsonDecode(sessionFile.readAsStringSync()) as Map<String, dynamic>;
      expect(sessionDoc['closed'], isTrue);

      // Each minted candidate is a record file whose canonical text is
      // the verbatim surface answer (I-3 end to end).
      final recordFiles = Directory(registry.recordsDir.path)
          .listSync(recursive: true)
          .whereType<File>()
          .where((f) => f.path.endsWith('.json'))
          .toList();
      expect(recordFiles, hasLength(2));
      final canonicalTexts = recordFiles.map((f) {
        final doc = jsonDecode(f.readAsStringSync()) as Map<String, dynamic>;
        return doc['canonical_text'];
      }).toSet();
      expect(canonicalTexts,
          contains('We went to the lake every summer.'));

      // Landing never approves: the gate log stays empty; statuses stay
      // candidate.
      for (final f in recordFiles) {
        final doc = jsonDecode(f.readAsStringSync()) as Map<String, dynamic>;
        expect(doc['status'], RecordStatus.candidate.name);
      }
      final gateCount = registry.gateLog().length;
      expect(gateCount, 0);

      // Re-landing the grown session is a refresh, not a duplication.
      final before = recordFiles.length;
      controller.land(sessionClosed: true);
      final after = Directory(registry.recordsDir.path)
          .listSync(recursive: true)
          .whereType<File>()
          .where((f) => f.path.endsWith('.json'))
          .length;
      expect(after, before);
    });
  });
}
