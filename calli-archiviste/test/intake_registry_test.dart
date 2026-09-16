/// Intake registry checks — Stage 5-b-2 (design §8 item 7 + gate contract).
///
/// Covers the three write paths and the guarantees that make the registry
/// trustworthy:
///   - files are truth, sqlite is a rebuildable index (rebuild is
///     idempotent and mirrors whatever the files say);
///   - gate_log is the one non-rebuildable table: rebuild preserves it
///     byte-for-byte in content;
///   - gate accepts exactly one actor (dave); denied attempts are logged
///     and touch nothing else;
///   - close requires approval (I-2); closure lives in the superset
///     `closure` block, mirrored in the index as status `closed`;
///   - unknown fields in record files are preserved, never dropped
///     (PKC_LAYOUT_SPEC §5).
///
/// Windows note: `flutter test` needs a loadable sqlite3 DLL. The
/// `registerSqliteOverride()` call below searches `SAM_SQLITE3_PATH`, the
/// bare system/PATH names, and the package + `test/` folders. If nothing
/// loads, download the sqlite.org "sqlite-dll-win-x64" zip and drop
/// `sqlite3.dll` next to `pubspec.yaml`; the failure message lists every
/// path it tried.
library;

import 'dart:convert';
import 'dart:io';

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/intake_registry.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  registerSqliteOverride();

  late Directory tmp;
  late IntakeRegistry reg;
  var idCounter = 0;

  DateTime fixedClock() => DateTime.utc(2026, 9, 16, 12);

  DefaultProbeChainEngine makeEngine() => DefaultProbeChainEngine(
        idFactory: () => 'rc-test-${idCounter++}',
        clock: fixedClock,
      );

  IntakeRegistry open() => IntakeRegistry.open(
        recordsDir: Directory('${tmp.path}/records'),
        registryDir: Directory('${tmp.path}/registry'),
        clock: fixedClock,
      );

  setUp(() {
    tmp = Directory.systemTemp.createTempSync('intake_registry_test');
    idCounter = 0;
    reg = open();
  });

  tearDown(() {
    reg.close();
    tmp.deleteSync(recursive: true);
  });

  /// A full L1->L5 chain with substantive answers at every level.
  ProbeChainState fullChain(DefaultProbeChainEngine engine) {
    final state = engine.start(
      mode: ElicitationMode.freeRecall,
      topic: 'the porch swing',
    );
    engine.next(state, 'We sat on the porch swing every summer evening.');
    engine.next(state, 'The chains creaked and the pines smelled sharp.');
    engine.next(state, 'I was there; I heard the creak myself.');
    engine.next(state, 'It was quieter than the neighbor radio, always.');
    engine.next(state, 'Because it was the only hour the house stopped.');
    return state;
  }

  /// Harvest helper: closure + mint + append in one step.
  SessionAppendReport harvest(
    IntakeRegistry registry,
    DefaultProbeChainEngine engine,
    ProbeChainState state, {
    bool closed = true,
  }) {
    return registry.sessionAppend(
      state: state,
      closureReport: engine.closureStatus(state),
      minted: engine.mint(state),
      sessionClosed: closed,
    );
  }

  group('schema + open', () {
    test('open creates dirs, schema, meta version, empty counts', () {
      expect(
        File('${tmp.path}/registry/registry.sqlite').existsSync(),
        isTrue,
      );
      expect(Directory('${tmp.path}/registry/sessions').existsSync(), isTrue);
      expect(reg.counts(), equals({
        'records': 0,
        'sessions': 0,
        'probe_closure': 0,
        'gate_log': 0,
        'records_fts': 0,
      }));
    });
  });

  group('write path 1: session append', () {
    test('full chain lands session rows, probe_closure, records, FTS', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);

      expect(report.mintedRecordIds.length, equals(2));
      expect(reg.counts(), equals({
        'records': 2,
        'sessions': 1,
        'probe_closure': 1,
        'gate_log': 0,
        'records_fts': 2,
      }));
      // The chain reached meaning unpoisoned: self-closable.
      final hits = reg.search('porch');
      expect(hits.length, equals(2));
      expect(hits.every((h) => h.status == 'candidate'), isTrue);
    });

    test('minted files use the PKC superset dialect', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);

      final hit = reg
          .search('porch')
          .firstWhere((h) => h.recordId == report.mintedRecordIds.first);
      final doc = reg.readRecord(hit.recordId)!;
      expect(doc['record_id'], equals(hit.recordId));
      expect(doc['status'], equals('candidate'));
      expect(doc['pkc_tier'], equals('S1'));
      final provenance = doc['provenance'] as Map;
      final elicitation = provenance['elicitation'] as Map;
      expect(elicitation['mode'], equals('free_recall'));
      expect(elicitation['session_id'], equals('@session:${state.sessionId}'));
      expect(elicitation['probe_chain'] as List, isNotEmpty);
      final source = (doc['sources'] as List).first as Map;
      expect(source['locator'].toString(), startsWith('@session:'));
      final closure = doc['closure'] as Map;
      expect(closure['closed'], equals(false));
      expect(closure['closed_by'], equals(''));
    });

    test('re-append upserts the session and merges minted ids', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      harvest(reg, engine, state);

      // The Bench may re-land the session state as it grows; no new mints.
      reg.sessionAppend(
        state: state,
        closureReport: engine.closureStatus(state),
        minted: const [],
      );
      expect(reg.counts()['sessions'], equals(1));
      expect(reg.counts()['records'], equals(2));

      // session file keeps the merged minted list
      final sessionDoc = jsonDecode(
        File(
          '${tmp.path}/registry/sessions/${_sessionIdOf(reg)}.json',
        ).readAsStringSync(),
      ) as Map;
      expect((sessionDoc['minted_record_ids'] as List).length, equals(2));
      // closure rows follow the session through the upsert
      expect(reg.probeClosure(state.sessionId)!['self_close_eligible'], isTrue);
      expect(reg.probeClosure(state.sessionId)!['poisoned'], isFalse);
    });
  });

  group('search', () {
    test('FTS hit, miss, and multi-token AND', () {
      final engine = makeEngine();
      harvest(reg, engine, fullChain(engine));

      expect(reg.search('porch'), isNotEmpty);
      expect(reg.search('zzznotthere'), isEmpty);
      // AND semantics within one canonical text
      expect(reg.search('porch swing'), isNotEmpty);
      // AND semantics across answers: 'creaked' lives in the sensory
      // answer, which is no record's canonical_text — no hits.
      expect(reg.search('porch creaked'), isEmpty);
      // sanitize: match-syntax characters must not throw or widen
      expect(reg.search('porch" * (swing)'), isNotEmpty);
      expect(reg.search('   '), isEmpty);
    });

    test('gate re-index does not duplicate FTS rows', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final eventId = report.mintedRecordIds.first;

      reg.gate(
        recordId: eventId,
        action: GateAction.approve,
        actor: GatePolicy.approvedActor,
      );
      final hits = reg.search('porch').where((h) => h.recordId == eventId);
      expect(hits.length, equals(1));
      expect(hits.first.status, equals('approved'));
    });
  });

  group('write path 2: the gate', () {
    test('denies every actor but dave, logs the denial, touches nothing',
        () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final eventId = report.mintedRecordIds.first;
      final before = reg.readRecord(eventId)!;
      final gateRowsBefore = reg.counts()['gate_log'];

      final outcome = reg.gate(
        recordId: eventId,
        action: GateAction.approve,
        actor: 'engine',
      );
      expect(outcome.accepted, isFalse);

      expect(reg.readRecord(eventId)!, equals(before));
      final rows = reg.gateLog();
      expect(rows.length, equals(gateRowsBefore! + 1));
      expect(rows.last.note, contains('DENIED'));
      expect(rows.last.actor, equals('engine'));
    });

    test('close before approve is denied (I-2)', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final eventId = report.mintedRecordIds.first;

      final outcome = reg.gate(
        recordId: eventId,
        action: GateAction.close,
        actor: GatePolicy.approvedActor,
      );
      expect(outcome.accepted, isFalse);
      expect(
        reg.readRecord(eventId)!['closure'],
        equals({'closed': false, 'closed_by': ''}),
      );
    });

    test('dave approve then close: file, row, and stamp agree', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final eventId = report.mintedRecordIds.first;

      expect(
        reg
            .gate(
              recordId: eventId,
              action: GateAction.approve,
              actor: GatePolicy.approvedActor,
            )
            .accepted,
        isTrue,
      );
      final afterApprove = reg.readRecord(eventId)!;
      expect(afterApprove['status'], equals('approved'));
      expect(
        reg.search('porch').firstWhere((h) => h.recordId == eventId).status,
        equals('approved'),
      );

      expect(
        reg
            .gate(
              recordId: eventId,
              action: GateAction.close,
              actor: GatePolicy.approvedActor,
              note: 'chain reached meaning, verdict mine',
            )
            .accepted,
        isTrue,
      );
      final afterClose = reg.readRecord(eventId)!;
      // Superset law: closed is a closure block, not a status change.
      expect(afterClose['status'], equals('approved'));
      final closure = afterClose['closure'] as Map;
      expect(closure['closed'], equals(true));
      expect(closure['closed_by'], equals('dave'));
      // Index mirror: status 'closed'.
      expect(
        reg.search('porch').firstWhere((h) => h.recordId == eventId).status,
        equals('closed'),
      );

      final stamps = reg.gateLog();
      expect(stamps.length, equals(2));
      expect(stamps.last.action, equals('close'));
      expect(stamps.last.note, contains('verdict mine'));
    });

    test('reopen from closed clears the closure block', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final eventId = report.mintedRecordIds.first;
      final dave = GatePolicy.approvedActor;

      reg.gate(recordId: eventId, action: GateAction.approve, actor: dave);
      reg.gate(recordId: eventId, action: GateAction.close, actor: dave);
      final reopenOutcome = reg.gate(
        recordId: eventId,
        action: GateAction.reopen,
        actor: dave,
      );
      expect(reopenOutcome.accepted, isTrue);

      final doc = reg.readRecord(eventId)!;
      expect(doc['status'], equals('reopened'));
      expect((doc['closure'] as Map)['closed'], equals(false));
      // ...and dave can approve again after reopening.
      expect(
        reg.gate(recordId: eventId, action: GateAction.approve, actor: dave)
            .accepted,
        isTrue,
      );
    });

    test('gating an unindexed record fails loudly', () {
      var threw = false;
      try {
        reg.gate(
          recordId: 'rc-never-was',
          action: GateAction.approve,
          actor: GatePolicy.approvedActor,
        );
      } catch (_) {
        threw = true;
      }
      expect(threw, isTrue);
    });
  });

  group('write path 3: rebuild', () {
    test('rebuild is idempotent and search results match', () {
      final engine = makeEngine();
      final poisoned = engine.start(
        mode: ElicitationMode.freeRecall,
        topic: 'the lake house',
      );
      engine.next(poisoned, 'We swam until the lifeguard blew the whistle.');
      engine.skip(poisoned, 'I do not remember the detail.');
      harvest(reg, engine, poisoned);

      final first = reg.rebuild();
      final counts1 = reg.counts();
      final hits1 = reg.search('swam');
      final second = reg.rebuild();
      final counts2 = reg.counts();
      final hits2 = reg.search('swam');

      expect(counts1, equals(counts2));
      expect(first.records, equals(second.records));
      expect(first.gateLogPreserved, equals(second.gateLogPreserved));
      expect(hits1.map((h) => h.recordId), equals(hits2.map((h) => h.recordId)));
      // skip at sensory (L2) poisons closure: the poison must survive the
      // rebuild via the session file.
      expect(reg.counts()['probe_closure'], equals(1));
      expect(reg.probeClosure('no-such-session'), isNull);
    });

    test('poison marker survives rebuild via the session file', () {
      final engine = makeEngine();
      final poisoned = engine.start(
        mode: ElicitationMode.freeRecall,
        topic: 'the lake house',
      );
      engine.next(poisoned, 'We swam until the lifeguard blew the whistle.');
      engine.skip(poisoned, 'I do not remember the detail.');
      harvest(reg, engine, poisoned);

      final before = reg.probeClosure(poisoned.sessionId)!;
      expect(before['poisoned'], isTrue);
      expect(before['self_close_eligible'], isFalse);

      reg.rebuild();
      final after = reg.probeClosure(poisoned.sessionId)!;
      expect(after, equals(before));
    });

    test('rebuild never touches gate_log (design section 5)', () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final dave = GatePolicy.approvedActor;

      // One accepted stamp and one logged denial — then rebuild must keep
      // both, byte-for-byte in content.
      reg.gate(
        recordId: report.mintedRecordIds[0],
        action: GateAction.approve,
        actor: dave,
      );
      final denied = reg.gate(
        recordId: report.mintedRecordIds[1],
        action: GateAction.approve,
        actor: 'model',
      );
      expect(denied.accepted, isFalse);

      final stampsBefore = reg.gateLog();
      expect(stampsBefore.length, equals(2));
      reg.rebuild();
      final stampsAfter = reg.gateLog();

      expect(stampsAfter.length, equals(stampsBefore.length));
      for (var i = 0; i < stampsBefore.length; i++) {
        expect(stampsAfter[i].ts, equals(stampsBefore[i].ts));
        expect(stampsAfter[i].recordId, equals(stampsBefore[i].recordId));
        expect(stampsAfter[i].action, equals(stampsBefore[i].action));
        expect(stampsAfter[i].actor, equals(stampsBefore[i].actor));
        expect(stampsAfter[i].note, equals(stampsBefore[i].note));
      }
    });

    test('files are truth: edits + unknown fields survive gate and rebuild',
        () {
      final engine = makeEngine();
      final state = fullChain(engine);
      final report = harvest(reg, engine, state);
      final eventId = report.mintedRecordIds.first;
      final file = File('${tmp.path}/records/events/$eventId.json');

      // Dave (or any earlier tool) hand-annotates the file with an unknown
      // field and resets the status: the registry must preserve and mirror.
      final doc =
          jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
      doc['status'] = 'candidate';
      doc['daves_note'] = 'checked against the photo album, holds';
      file.writeAsStringSync(JsonEncoder.withIndent('  ').convert(doc));

      reg.rebuild();
      expect(
        reg.search('porch').firstWhere((h) => h.recordId == eventId).status,
        equals('candidate'),
      );

      // Gate the file: unknown field must survive the in-place mutation.
      reg.gate(
        recordId: eventId,
        action: GateAction.approve,
        actor: GatePolicy.approvedActor,
      );
      final afterGate =
          jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
      expect(afterGate['status'], equals('approved'));
      expect(afterGate['daves_note'], equals('checked against the photo album, holds'));
    });

    test('a lost index rebuilds itself from files on open', () {
      final engine = makeEngine();
      harvest(reg, engine, fullChain(engine));
      final stampsBefore = reg.counts();

      // Simulate the documented recovery: registry.sqlite deleted.
      reg.close();
      File('${tmp.path}/registry/registry.sqlite').deleteSync();
      reg = open();

      expect(reg.counts(), equals(stampsBefore));
      expect(reg.search('porch'), isNotEmpty);
    });
  });
}

String _sessionIdOf(IntakeRegistry registry) {
  // single-session helper for the upsert test: read the sessions dir
  final dir = Directory(
    '${registry.registryDir.path}/sessions',
  );
  return dir
      .listSync()
      .whereType<File>()
      .firstWhere((f) => f.path.endsWith('.json'))
      .uri
      .pathSegments
      .last
      .replaceAll('.json', '');
}
