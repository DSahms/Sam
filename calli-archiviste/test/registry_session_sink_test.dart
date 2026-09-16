/// RegistrySessionSink checks — Stage 5-b-3 hotfix.
///
/// The Bench wires [ChainSessionController] to [IntakeRegistry] through
/// this sink. These checks exist because the first 5-b-3 commit referenced
/// the sink without shipping it: the sandbox harness had its own bridge,
/// and the Bench app is Flutter-only, so the gap surfaced as a Windows
/// compile error instead of a sandbox test failure.
///
/// The tests drive the exact path the Bench UI drives —
/// controller -> sink -> registry -> files on disk — so the bridge can
/// never silently vanish or drift again:
///   - a full five-level chain lands through the sink: session file on
///     disk, record files on disk, gate_log untouched (I-4);
///   - re-landing is a state refresh, never duplicate records (mint-once).
///
/// Windows note: `flutter test` needs a loadable sqlite3 DLL; see the
/// note in intake_registry_test.dart — same search, same fix.
library;

import 'dart:convert';
import 'dart:io';

import 'package:calli_archiviste/services/intake_registry.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';
import 'package:calli_archiviste/services/registry_session_sink.dart';
import 'package:calli_archiviste/services/probe_session_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  registerSqliteOverride();

  late Directory tmp;
  late IntakeRegistry reg;
  var idCounter = 0;

  DateTime fixedClock() => DateTime.utc(2026, 9, 16, 12);

  DefaultProbeChainEngine makeEngine() => DefaultProbeChainEngine(
        idFactory: () => 'rc-sink-${idCounter++}',
        clock: fixedClock,
      );

  setUp(() {
    tmp = Directory.systemTemp.createTempSync('registry_sink_test');
    idCounter = 0;
    reg = IntakeRegistry.open(
      recordsDir: Directory('${tmp.path}/records'),
      registryDir: Directory('${tmp.path}/registry'),
      clock: fixedClock,
    );
  });

  tearDown(() {
    reg.close();
    tmp.deleteSync(recursive: true);
  });

  /// Drives the controller through a full L1->L5 chain with substantive
  /// answers, no wording service (static bank; zero model calls), and
  /// lands it the way the Bench's "File the harvest" button does.
  ChainSessionController runFullChain(DefaultProbeChainEngine engine) {
    final controller = ChainSessionController(
      engine: engine,
      sink: RegistrySessionSink(reg),
    );
    return controller;
  }

  Future<ChainSessionController> begunController(
    DefaultProbeChainEngine engine,
  ) async {
    final controller = runFullChain(engine);
    await controller.begin(
      mode: ElicitationMode.freeRecall,
      topic: 'the porch swing',
    );
    await controller.submitAnswer(
        'We sat on the porch swing every summer evening.');
    await controller.submitAnswer(
        'The chains creaked and the pines smelled sharp.');
    await controller.submitAnswer('I was there; I heard the creak myself.');
    await controller.submitAnswer(
        'It was quieter than the neighbor radio, always.');
    await controller.submitAnswer(
        'Because it was the only hour the house stopped.');
    return controller;
  }

  group('RegistrySessionSink (bench bridge)', () {
    test('full chain lands through the sink: files exist, gate_log empty',
        () async {
      final controller = await begunController(makeEngine());
      final minted = controller.land(sessionClosed: true);

      expect(minted, isNotEmpty);
      expect(reg.counts()['sessions'], equals(1));
      expect(reg.counts()['records'], equals(minted.length));
      expect(reg.counts()['gate_log'], equals(0));

      // Exactly one session file, marked closed, carrying the minted ids.
      final sessionsDir = Directory('${tmp.path}/registry/sessions');
      final files = sessionsDir.listSync().whereType<File>().toList();
      expect(files.length, equals(1));
      final doc =
          jsonDecode(files.single.readAsStringSync()) as Map<dynamic, dynamic>;
      expect(doc['closed'], isTrue);
      expect((doc['minted_record_ids'] as List).length, equals(minted.length));

      // Every minted candidate is a real record file on disk (files are
      // truth — the sqlite index is rebuildable, these are not).
      for (final id in doc['minted_record_ids'] as List) {
        final hits = Directory('${tmp.path}/records')
            .listSync(recursive: true)
            .whereType<File>()
            .where((f) => f.readAsStringSync().contains('"$id"'))
            .toList();
        expect(hits, isNotEmpty, reason: 'record file missing for $id');
      }
    });

    test('re-land is a state refresh, never duplicate records', () async {
      final controller = await begunController(makeEngine());
      final minted = controller.land(sessionClosed: true);
      final recordsAfterFirstLand = reg.counts()['records'];
      expect(recordsAfterFirstLand, equals(minted.length));

      // The Bench may land again as state grows (mint-once semantics).
      final again = controller.land(sessionClosed: true);
      expect(again, isEmpty);
      expect(reg.counts()['records'], equals(recordsAfterFirstLand));
      expect(reg.counts()['sessions'], equals(1));
    });

    test('sink surfaces registry failures (no silent drops)', () {
      final sink = RegistrySessionSink(reg);
      // A state with no session id is invalid; the registry throws and the
      // sink must NOT swallow it — the Bench has to see a failed write.
      final bad = ProbeChainState(
        sessionId: '   ',
        mode: ElicitationMode.freeRecall,
        topic: 'x',
      );
      expect(
        () => sink.append(state: bad),
        throwsArgumentError,
      );
    });
  });
}
