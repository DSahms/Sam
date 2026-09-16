/// The intake registry — Stage 5-b-2.
///
/// Implements the registry contract from
/// docs/architecture/stage-5b-intake-registry.md (§5 schema, §6 gate) and
/// the record superset from PKC_LAYOUT_SPEC.md r2.1 (§5, §7).
///
/// REGISTRY PHILOSOPHY: **the JSON files are truth, sqlite is speed.**
/// `registry.sqlite` is a rebuildable index over the record files; deleting
/// it loses nothing but speed. The one exception is `gate_log` — the only
/// table that is NOT rebuildable and is never exported (design §5). It is
/// the stamp album: every approval, rejection, reopen, close, and every
/// denied attempt, recorded forever.
///
/// WRITE PATHS ARE EXACTLY THREE (design §5):
///   1. [sessionAppend]  — a probe chain session lands (state + closure +
///      minted candidates as record files + rows).
///   2. [gate]           — a gate action lands (JSON status + gate_log in
///      one step; denied attempts are logged and touch nothing else).
///   3. [rebuild]        — the index is reconstructed from the files,
///      idempotently, never touching `gate_log`.
///
/// FILE FORMAT (PKC_LAYOUT_SPEC §5 superset): record files carry the full
/// superset dialect — `provenance.elicitation.{mode,session_id,probe_chain}`,
/// `sources[].{locator,sha256,mtime}`, `closure.{closed,closed_by}` — and
/// **unknown fields are preserved, never dropped**: gate actions mutate the
/// decoded map in place, so hand-registered fields survive every rewrite.
/// The engine's [RecordCandidate] is a projection; the registry projects it
/// INTO the superset on mint, and maps closure back to the engine's
/// `RecordStatus.closed` on read.
///
/// CRASH SEMANTICS OF THE GATE (documented ordering): the JSON file is
/// written first (atomic tmp+rename), then the sqlite transaction (records
/// row + gate_log). A crash between the two leaves the file showing the new
/// status with a missing gate_log stamp; the next [rebuild] reconciles the
/// index to the file (files are truth) and Dave can re-stamp. The reverse
/// ordering would leave a stamp contradicted by the file — strictly worse,
/// because a stamp is a decision and a file is recoverable.
///
/// Pure Dart + sqlite. No model calls, no F: paths, no Flutter imports.
/// The sqlite library must be loadable in the host process (app bootstrap
/// wires `sqlite3_flutter_libs`; tests wire `open.overrideFor`).
library;

import 'dart:convert';
import 'dart:ffi' show DynamicLibrary;
import 'dart:io';

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';
import 'package:crypto/crypto.dart';
import 'package:sqlite3/sqlite3.dart';
import 'package:sqlite3/open.dart' as sqlite_open;

/// The intake registry. See the library docs for the contract.
class IntakeRegistry {
  IntakeRegistry._({
    required this.recordsDir,
    required this.registryDir,
    required Database db,
    DateTime Function()? clock,
  })  : _db = db,
        _clock = clock;

  /// Storage schema version for files and for `meta.schema_version`.
  /// Bump only when a field changes meaning, not when fields are added.
  static const int schemaVersion = 1;

  /// Where record JSON files live (PKC §3: `02_records/`).
  final Directory recordsDir;

  /// Where registry.sqlite and session files live (PKC §3: `05_registry/`).
  final Directory registryDir;

  final Database _db;
  final DateTime Function()? _clock;

  Directory get _sessionsDir =>
      Directory('${registryDir.path}/sessions');

  String get _sqlitePath => '${registryDir.path}/registry.sqlite';

  DateTime _now() => (_clock?.call() ?? DateTime.now()).toUtc();

  String _nowIso() => _now().toIso8601String();

  // -------------------------------------------------------------------------
  // open + schema
  // -------------------------------------------------------------------------

  /// Opens (creating if needed) the registry at [recordsDir]/[registryDir].
  ///
  /// If the sqlite index is missing or empty while record files exist, a
  /// [rebuild] runs immediately: a fresh or damaged index is the documented
  /// recovery path, and rebuild is idempotent (design §8.7).
  factory IntakeRegistry.open({
    required Directory recordsDir,
    required Directory registryDir,
    DateTime Function()? clock,
  }) {
    registryDir.createSync(recursive: true);
    final db = sqlite3.open('${registryDir.path}/registry.sqlite');
    final reg = IntakeRegistry._(
      recordsDir: recordsDir,
      registryDir: registryDir,
      db: db,
      clock: clock,
    );
    try {
      reg._ensureSchema();
      final files = reg._recordFiles().length;
      final rows = db.select('SELECT COUNT(*) AS n FROM records').first['n']
          as int;
      if (files > 0 && rows == 0) {
        reg.rebuild();
      }
    } catch (_) {
      db.dispose();
      rethrow;
    }
    return reg;
  }

  void _ensureSchema() {
    // The registry lays out its whole tree at open: an inspectable corpus
    // from the first moment, even before the first session lands.
    _sessionsDir.createSync(recursive: true);
    _db.execute('''
      CREATE TABLE IF NOT EXISTS meta(
        key TEXT PRIMARY KEY,
        value TEXT
      )
    ''');
    _db.execute('''
      CREATE TABLE IF NOT EXISTS sessions(
        session_id TEXT PRIMARY KEY,
        mode TEXT,
        topic TEXT,
        started_at TEXT,
        closed INTEGER,
        closure_report TEXT
      )
    ''');
    _db.execute('''
      CREATE TABLE IF NOT EXISTS records(
        record_id TEXT PRIMARY KEY,
        record_type TEXT,
        status TEXT,
        canonical_text TEXT,
        sensitivity TEXT,
        pkc_tier TEXT,
        confidence REAL,
        source_sha256 TEXT,
        file_mtime TEXT,
        json_path TEXT
      )
    ''');
    _db.execute('''
      CREATE VIRTUAL TABLE IF NOT EXISTS records_fts USING fts5(
        record_id UNINDEXED,
        canonical_text,
        tokenize='porter unicode61'
      )
    ''');
    _db.execute('''
      CREATE TABLE IF NOT EXISTS mentions(
        record_id TEXT,
        mention_of TEXT,
        kind TEXT
      )
    ''');
    _db.execute('''
      CREATE TABLE IF NOT EXISTS relationships(
        from_id TEXT,
        to_id TEXT,
        rel_type TEXT
      )
    ''');
    _db.execute('''
      CREATE TABLE IF NOT EXISTS probe_closure(
        session_id TEXT PRIMARY KEY,
        reached_level TEXT,
        poisoned INTEGER,
        self_close_eligible INTEGER
      )
    ''');
    _db.execute('''
      CREATE TABLE IF NOT EXISTS gate_log(
        ts TEXT,
        record_id TEXT,
        action TEXT,
        actor TEXT,
        note TEXT
      )
    ''');
    _db.execute(
      'INSERT OR IGNORE INTO meta(key, value) VALUES (?, ?)',
      ['schema_version', '$schemaVersion'],
    );
    _db.execute(
      'INSERT OR IGNORE INTO meta(key, value) VALUES (?, ?)',
      ['created_at', _nowIso()],
    );
  }

  /// Closes the sqlite handle. The files stay canonical; reopening is safe.
  void close() => _db.dispose();

  // -------------------------------------------------------------------------
  // write path 1: session append
  // -------------------------------------------------------------------------

  /// Lands a probe chain session: the lossless state file, the sessions
  /// index row, the probe_closure row, and (when [minted] is non-empty) the
  /// minted candidates as superset record files + rows + FTS entries.
  ///
  /// This is the ONLY place engine-minted records become files. Re-append
  /// is an upsert: a session that grows re-lands its state; minted records
  /// keep their file as first written (candidates are immutable until the
  /// gate).
  SessionAppendReport sessionAppend({
    required ProbeChainState state,
    ClosureReport? closureReport,
    List<RecordCandidate> minted = const [],
    bool sessionClosed = false,
  }) {
    if (state.sessionId.trim().isEmpty) {
      throw ArgumentError('session state needs a session_id');
    }
    _sessionsDir.createSync(recursive: true);

    final effectiveClosure =
        closureReport ?? DefaultProbeChainEngine().closureStatus(state);
    final mintedIds = minted.map((c) => c.recordId).toList(growable: false);

    // 1) session file (canonical state)
    final sessionFile =
        File('${_sessionsDir.path}/${state.sessionId}.json');
    final existing = _readJsonMapOrNull(sessionFile);
    final alreadyMinted = existing == null
        ? <String>{}
        : (existing['minted_record_ids'] as List? ?? const <String>[])
            .whereType<String>()
            .toSet();
    final sessionDoc = <String, Object?>{
      'schema_version': schemaVersion,
      'state': state.toJson(),
      'closed': sessionClosed,
      'closure_report': {
        'self_close_eligible': effectiveClosure.selfCloseEligible,
        'reason': effectiveClosure.reason,
        'reached_contrast': effectiveClosure.reachedContrast,
        'poisoned_by_skip': effectiveClosure.poisonedBySkip,
      },
      'minted_record_ids': <String>{...alreadyMinted, ...mintedIds}.toList(),
      'appended_at': _nowIso(),
    };
    _atomicWriteJson(sessionFile, sessionDoc);

    // 2) record files for freshly minted candidates (before rows, so the
    //    transaction below never indexes a missing file)
    final written = <String>[];
    for (final candidate in minted) {
      if (alreadyMinted.contains(candidate.recordId)) continue;
      final relPath = _mintPath(candidate);
      final file = File('${recordsDir.path}/$relPath');
      file.parent.createSync(recursive: true);
      if (!file.existsSync()) {
        _atomicWriteJson(
          file,
          _supersetFromCandidate(candidate, state: state),
        );
      }
      written.add(relPath);
    }

    // 3) sqlite rows in one transaction
    _db.execute('BEGIN');
    try {
      _db.execute(
        '''
        INSERT OR REPLACE INTO sessions(
          session_id, mode, topic, started_at, closed, closure_report
        ) VALUES (?, ?, ?, ?, ?, ?)
        ''',
        [
          state.sessionId,
          state.mode.wireValue,
          state.topic,
          state.startedAt.toIso8601String(),
          sessionClosed ? 1 : 0,
          jsonEncode(sessionDoc['closure_report']),
        ],
      );
      final report = effectiveClosure;
      _db.execute(
        '''
        INSERT OR REPLACE INTO probe_closure(
          session_id, reached_level, poisoned, self_close_eligible
        ) VALUES (?, ?, ?, ?)
        ''',
        [
          state.sessionId,
          state.reachedLevel.wireValue,
          report.poisonedBySkip ? 1 : 0,
          report.selfCloseEligible ? 1 : 0,
        ],
      );
      for (var i = 0; i < minted.length; i++) {
        final candidate = minted[i];
        final relPath = _mintPath(candidate);
        final file = File('${recordsDir.path}/$relPath');
        _upsertRecordRow(file, relPath);
      }
      _db.execute('COMMIT');
    } catch (_) {
      _db.execute('ROLLBACK');
      rethrow;
    }

    return SessionAppendReport(
      sessionId: state.sessionId,
      mintedRecordIds: mintedIds,
      mintedPaths: List.unmodifiable(written),
      sessionFile: sessionFile.path,
    );
  }

  // -------------------------------------------------------------------------
  // write path 2: the gate
  // -------------------------------------------------------------------------

  /// The gate (design §6). `actor != 'dave'` is denied, always, with the
  /// reason recorded in gate_log. `close` requires an approved record (I-2);
  /// in the superset dialect that is `closure.closed=true, closed_by=actor`
  /// with the file status staying `approved`, mirrored in the index as
  /// status `closed`.
  ///
  /// Denied attempts: logged, nothing else touched. Accepted actions: JSON
  /// file first (atomic), then one sqlite transaction (row + gate_log).
  GateOutcome gate({
    required String recordId,
    required GateAction action,
    required String actor,
    String? note,
  }) {
    final row = _db
        .select('SELECT json_path FROM records WHERE record_id = ?', [recordId]);
    if (row.isEmpty) {
      throw StateError(
        'record "$recordId" is not indexed; run rebuild() or append the '
        'session that minted it before gating',
      );
    }
    final relPath = row.first['json_path'] as String;
    final file = _resolveRecordFile(relPath);
    final doc = _readJsonMapOrNull(file);
    if (doc == null) {
      throw StateError(
        'indexed record "$recordId" is missing its file: $relPath '
        '(files are truth — restore the file, then rebuild)',
      );
    }

    final decision = GatePolicy.evaluate(
      actor: actor,
      action: action,
      currentStatus: _engineStatusFor(doc),
      recordType: _engineTypeFor(doc),
    );

    final ts = _nowIso();
    if (!decision.allowed) {
      _db.execute(
        'INSERT INTO gate_log(ts, record_id, action, actor, note) '
        'VALUES (?, ?, ?, ?, ?)',
        [
          ts,
          recordId,
          action.name,
          actor,
          'DENIED: ${decision.reason ?? 'denied'}',
        ],
      );
      return GateOutcome.denied(
        recordId: recordId,
        action: action,
        actor: actor,
        reason: decision.reason ?? 'denied',
        ts: ts,
      );
    }

    // 1) mutate the decoded map in place (unknown fields survive) and write.
    _applyGateToFile(doc, action, actor: actor);
    _atomicWriteJson(file, doc);

    // 2) one transaction: index row + the stamp.
    _db.execute('BEGIN');
    try {
      _upsertRecordRow(file, relPath);
      _db.execute(
        'INSERT INTO gate_log(ts, record_id, action, actor, note) '
        'VALUES (?, ?, ?, ?, ?)',
        [ts, recordId, action.name, actor, note ?? ''],
      );
      _db.execute('COMMIT');
    } catch (_) {
      _db.execute('ROLLBACK');
      rethrow;
    }

    return GateOutcome.accepted(
      recordId: recordId,
      action: action,
      actor: actor,
      ts: ts,
    );
  }

  void _applyGateToFile(
    Map<String, Object?> doc,
    GateAction action, {
    required String actor,
  }) {
    switch (action) {
      case GateAction.approve:
        doc['status'] = RecordStatus.approved.name;
        break;
      case GateAction.reject:
        doc['status'] = RecordStatus.rejected.name;
        break;
      case GateAction.reopen:
        doc['status'] = RecordStatus.reopened.name;
        (doc['closure'] as Map<String, Object?>? ??
                _ensureClosureBlock(doc))['closed'] = false;
        (doc['closure'] as Map<String, Object?>)['closed_by'] = '';
        break;
      case GateAction.close:
        // I-2: policy already required engine status == approved. The file
        // keeps status approved; closedness lives in the closure block.
        final closure = doc['closure'] as Map<String, Object?>? ??
            _ensureClosureBlock(doc);
        closure['closed'] = true;
        closure['closed_by'] = actor;
        break;
    }
  }

  Map<String, Object?> _ensureClosureBlock(Map<String, Object?> doc) {
    final existing = doc['closure'];
    if (existing is Map<String, Object?>) return existing;
    final fresh = <String, Object?>{'closed': false, 'closed_by': ''};
    doc['closure'] = fresh;
    return fresh;
  }

  // -------------------------------------------------------------------------
  // write path 3: rebuild
  // -------------------------------------------------------------------------

  /// Reconstructs the entire index from the JSON files. Idempotent: two
  /// rebuilds in a row leave identical row counts (design §8.7). `gate_log`
  /// is NEVER touched — it is the one non-rebuildable table (design §5).
  RebuildReport rebuild() {
    final gateRowsBefore =
        _db.select('SELECT COUNT(*) AS n FROM gate_log').first['n'] as int;

    _db.execute('BEGIN');
    try {
      _db.execute('DELETE FROM sessions');
      _db.execute('DELETE FROM probe_closure');
      _db.execute('DELETE FROM records');
      _db.execute('DELETE FROM records_fts');
      // mentions/relationships have no writers yet (claims land later);
      // cleared for the same idempotency guarantee.
      _db.execute('DELETE FROM mentions');
      _db.execute('DELETE FROM relationships');
      _db.execute('COMMIT');
    } catch (_) {
      _db.execute('ROLLBACK');
      rethrow;
    }

    var recordCount = 0;
    for (final file in _recordFiles()) {
      final relPath = _relativeToRecords(file);
      _upsertRecordRow(file, relPath);
      recordCount++;
    }

    var sessionCount = 0;
    if (_sessionsDir.existsSync()) {
      for (final entity in _sessionsDir.listSync().whereType<File>()) {
        if (!entity.path.endsWith('.json')) continue;
        final doc = _readJsonMapOrNull(entity);
        if (doc == null) continue;
        _upsertSessionRow(entity, doc);
        sessionCount++;
      }
    }

    final ts = _nowIso();
    _db.execute(
      'INSERT OR REPLACE INTO meta(key, value) VALUES (?, ?)',
      ['last_rebuild', ts],
    );

    final gateRowsAfter =
        _db.select('SELECT COUNT(*) AS n FROM gate_log').first['n'] as int;
    if (gateRowsAfter != gateRowsBefore) {
      throw StateError(
        'rebuild changed gate_log ($gateRowsBefore -> $gateRowsAfter): '
        'this is a contract violation — gate_log is never rebuildable',
      );
    }

    return RebuildReport(
      records: recordCount,
      sessions: sessionCount,
      gateLogPreserved: gateRowsAfter,
      at: ts,
    );
  }

  // -------------------------------------------------------------------------
  // reads (for the Bench harvest view, 5-b-3/5-b-4)
  // -------------------------------------------------------------------------

  /// The full superset document for [recordId], or null when not indexed.
  Map<String, Object?>? readRecord(String recordId) {
    final row = _db.select(
      'SELECT json_path FROM records WHERE record_id = ?',
      [recordId],
    );
    if (row.isEmpty) return null;
    return _readJsonMapOrNull(
      _resolveRecordFile(row.first['json_path'] as String),
    );
  }

  /// gate_log rows, newest last, for the 5-b-4 viewer. Read-only by design;
  /// there is no API here that edits or deletes stamps.
  List<GateLogRow> gateLog({int limit = 200}) {
    final rows = _db.select(
      'SELECT ts, record_id, action, actor, note FROM gate_log '
      'ORDER BY rowid DESC LIMIT ?',
      [limit],
    );
    return [
      for (final r in rows)
        GateLogRow(
          ts: r['ts'] as String? ?? '',
          recordId: r['record_id'] as String? ?? '',
          action: r['action'] as String? ?? '',
          actor: r['actor'] as String? ?? '',
          note: r['note'] as String? ?? '',
        ),
    ].reversed.toList(growable: false);
  }

  /// FTS5 search over canonical_text (porter + unicode61). Query tokens are
  /// quoted individually and joined with AND; a query that sanitizes to
  /// nothing returns no hits rather than throwing.
  List<RegistrySearchHit> search(String query, {int limit = 25}) {
    final match = _ftsMatchQuery(query);
    if (match == null) return const [];
    final rows = _db.select(
      '''
      SELECT r.record_id AS record_id,
             r.record_type AS record_type,
             r.status AS status,
             r.canonical_text AS canonical_text,
             r.confidence AS confidence,
             bm25(records_fts) AS rank
      FROM records_fts
      JOIN records r ON r.record_id = records_fts.record_id
      WHERE records_fts MATCH ?
      ORDER BY rank
      LIMIT ?
      ''',
      [match, limit],
    );
    return [
      for (final r in rows)
        RegistrySearchHit(
          recordId: r['record_id'] as String? ?? '',
          recordType: r['record_type'] as String? ?? '',
          status: r['status'] as String? ?? '',
          canonicalText: r['canonical_text'] as String? ?? '',
          confidence: (r['confidence'] as num?)?.toDouble() ?? 0,
          rank: (r['rank'] as num?)?.toDouble() ?? 0,
        ),
    ];
  }

  /// The probe_closure row for [sessionId] (design §5), or null when the
  /// session is unknown. The 5-b-4 gate viewer shows this next to a record
  /// so Dave can see whether the chain behind it was poisoned.
  Map<String, Object?>? probeClosure(String sessionId) {
    final rows = _db.select(
      'SELECT session_id, reached_level, poisoned, self_close_eligible '
      'FROM probe_closure WHERE session_id = ?',
      [sessionId],
    );
    if (rows.isEmpty) return null;
    final r = rows.first;
    return {
      'session_id': r['session_id'] as String? ?? '',
      'reached_level': r['reached_level'] as String? ?? '',
      'poisoned': (r['poisoned'] as int? ?? 0) == 1,
      'self_close_eligible': (r['self_close_eligible'] as int? ?? 0) == 1,
    };
  }

  /// Row counts used by tests and cheap health checks.
  Map<String, int> counts() {
    int one(String sql) => _db.select(sql).first['n'] as int;
    return {
      'records': one('SELECT COUNT(*) AS n FROM records'),
      'sessions': one('SELECT COUNT(*) AS n FROM sessions'),
      'probe_closure': one('SELECT COUNT(*) AS n FROM probe_closure'),
      'gate_log': one('SELECT COUNT(*) AS n FROM gate_log'),
      'records_fts': one('SELECT COUNT(*) AS n FROM records_fts'),
    };
  }

  // -------------------------------------------------------------------------
  // projection: engine candidate -> superset file
  // -------------------------------------------------------------------------

  /// Projects a minted candidate into the PKC §5 superset dialect. The
  /// engine's flat projection becomes the corpus shape: provenance block,
  /// object sources, uppercase tier, closure block. I-4 guarantees the
  /// candidate status is `candidate`; anything else is a contract break.
  Map<String, Object?> _supersetFromCandidate(
    RecordCandidate candidate, {
    required ProbeChainState state,
  }) {
    if (candidate.status != RecordStatus.candidate) {
      throw ArgumentError(
        'mint produced a ${candidate.status.name}; I-4 says candidates only',
      );
    }
    return {
      'schema_version': schemaVersion,
      'record_id': candidate.recordId,
      'record_type': candidate.recordType.wireValue,
      'status': candidate.status.name,
      'canonical_text': candidate.canonicalText,
      'sensitivity': candidate.sensitivity.wireValue,
      'pkc_tier': candidate.pkcTier.name.toUpperCase(),
      'confidence': candidate.confidence,
      if (candidate.createdAt != null)
        'created_at': candidate.createdAt!.toIso8601String(),
      'provenance': {
        'engine': 'calli-archiviste 5-b-1 mint',
        'elicitation': {
          'mode': state.mode.wireValue,
          'session_id': '@session:${state.sessionId}',
          'probe_chain': candidate.probeChain,
        },
      },
      'sources': [
        for (final locator in candidate.sources)
          {'locator': locator, 'sha256': null, 'mtime': null},
      ],
      'closure': {'closed': false, 'closed_by': ''},
    };
  }

  /// Superset status -> engine status for [GatePolicy]. `closed` is not a
  /// file status: it is `closure.closed == true` (PKC §5), mirrored in the
  /// index as status `closed`.
  RecordStatus _engineStatusFor(Map<String, Object?> doc) {
    final closure = doc['closure'];
    final closed = closure is Map && closure['closed'] == true;
    if (closed) return RecordStatus.closed;
    switch (doc['status'] as String? ?? 'candidate') {
      case 'approved':
        return RecordStatus.approved;
      case 'rejected':
        return RecordStatus.rejected;
      case 'reopened':
        return RecordStatus.reopened;
      default:
        return RecordStatus.candidate;
    }
  }

  RecordType _engineTypeFor(Map<String, Object?> doc) {
    final wire = doc['record_type'] as String? ?? '';
    for (final t in RecordType.values) {
      if (t.wireValue == wire) return t;
    }
    // person / place / work (and any future superset type) hit the same
    // gate rules; the policy branches on status, not type.
    return RecordType.judgment;
  }

  String _mintPath(RecordCandidate candidate) {
    final subdir = switch (candidate.recordType) {
      RecordType.event => 'events',
      RecordType.judgment => 'judgments',
      RecordType.memoryCandidate => 'memory_candidates',
      // person/place/work are never engine-minted; mapped for completeness.
      _ => '${candidate.recordType.wireValue}s',
    };
    return '$subdir/${candidate.recordId}.json';
  }

  // -------------------------------------------------------------------------
  // row plumbing
  // -------------------------------------------------------------------------

  /// Mirrors a record file into the index: row + FTS. `source_sha256` and
  /// `file_mtime` are recomputed from the file every time it is indexed —
  /// the file is truth, the row follows it.
  void _upsertRecordRow(File file, String relPath) {
    final bytes = file.readAsBytesSync();
    final doc = _readJsonMapOrNull(file);
    if (doc == null) {
      throw FormatException('not a JSON object: ${file.path}');
    }
    final recordId = doc['record_id'] as String? ?? '';
    if (recordId.isEmpty) {
      throw FormatException('record file without record_id: ${file.path}');
    }
    final closure = doc['closure'];
    final closed = closure is Map && closure['closed'] == true;
    final status = closed
        ? RecordStatus.closed.name
        : (doc['status'] as String? ?? RecordStatus.candidate.name);

    _db.execute(
      '''
      INSERT OR REPLACE INTO records(
        record_id, record_type, status, canonical_text, sensitivity,
        pkc_tier, confidence, source_sha256, file_mtime, json_path
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ''',
      [
        recordId,
        doc['record_type'] as String? ?? '',
        status,
        doc['canonical_text'] as String? ?? '',
        doc['sensitivity'] as String? ?? 'normal',
        doc['pkc_tier'] as String? ?? 'S1',
        (doc['confidence'] as num?)?.toDouble() ?? 0,
        sha256.convert(bytes).toString(),
        file.lastModifiedSync().toUtc().toIso8601String(),
        relPath,
      ],
    );
    // Re-indexing an existing record (e.g. after a gate status change)
    // must not leave the old entry behind: delete-then-insert keeps one
    // FTS row per record, so rebuild and gate agree on hit counts.
    _db.execute(
      'DELETE FROM records_fts WHERE rowid IN '
      '(SELECT rowid FROM records_fts WHERE record_id = ?)',
      [recordId],
    );
    _db.execute(
      'INSERT INTO records_fts(record_id, canonical_text) VALUES (?, ?)',
      [recordId, doc['canonical_text'] as String? ?? ''],
    );
  }

  void _upsertSessionRow(File file, Map<String, Object?> doc) {
    final stateJson = doc['state'];
    if (stateJson is! Map) {
      throw FormatException('session file without state: ${file.path}');
    }
    final state = ProbeChainState.fromJson(
      Map<String, dynamic>.from(stateJson),
    );
    final closureJson = doc['closure_report'];
    ClosureReport report;
    if (closureJson is Map) {
      final c = Map<String, dynamic>.from(closureJson);
      report = ClosureReport(
        selfCloseEligible: c['self_close_eligible'] == true,
        reason: c['reason'] as String? ?? '',
        reachedContrast: c['reached_contrast'] == true,
        poisonedBySkip: c['poisoned_by_skip'] == true,
      );
    } else {
      report = DefaultProbeChainEngine().closureStatus(state);
    }
    final closed = doc['closed'] == true;

    _db.execute(
      '''
      INSERT OR REPLACE INTO sessions(
        session_id, mode, topic, started_at, closed, closure_report
      ) VALUES (?, ?, ?, ?, ?, ?)
      ''',
      [
        state.sessionId,
        state.mode.wireValue,
        state.topic,
        state.startedAt.toIso8601String(),
        closed ? 1 : 0,
        jsonEncode({
          'self_close_eligible': report.selfCloseEligible,
          'reason': report.reason,
          'reached_contrast': report.reachedContrast,
          'poisoned_by_skip': report.poisonedBySkip,
        }),
      ],
    );
    _db.execute(
      '''
      INSERT OR REPLACE INTO probe_closure(
        session_id, reached_level, poisoned, self_close_eligible
      ) VALUES (?, ?, ?, ?)
      ''',
      [
        state.sessionId,
        state.reachedLevel.wireValue,
        report.poisonedBySkip ? 1 : 0,
        report.selfCloseEligible ? 1 : 0,
      ],
    );
  }

  // -------------------------------------------------------------------------
  // file plumbing
  // -------------------------------------------------------------------------

  List<File> _recordFiles() {
    if (!recordsDir.existsSync()) return const [];
    return recordsDir
        .listSync(recursive: true)
        .whereType<File>()
        .where((f) => f.path.endsWith('.json'))
        .toList(growable: false);
  }

  String _relativeToRecords(File file) {
    // Normalize BOTH sides to forward slashes BEFORE the prefix match.
    // Windows: recordsDir.path may arrive with '/' (caller-built) while
    // Directory.listSync() returns '\' paths — a raw startsWith then fails
    // silently and the FULL path gets stored as json_path, which gate()
    // and readRecord() cannot resolve (first Windows run, 2026-09-16).
    var root = recordsDir.path.replaceAll('\\', '/');
    while (root.endsWith('/')) {
      root = root.substring(0, root.length - 1);
    }
    final path = file.path.replaceAll('\\', '/');
    if (path.startsWith('$root/')) {
      return path.substring(root.length + 1);
    }
    return path;
  }

  /// Resolves an indexed `json_path` against [recordsDir]. Indexes written
  /// before the separator fix may hold absolute paths; those are used
  /// verbatim (the next rebuild rewrites them as clean relative paths),
  /// so a stale index degrades to readable instead of broken.
  File _resolveRecordFile(String stored) {
    final normalized = stored.replaceAll('\\', '/');
    final looksAbsolute = normalized.startsWith('/') ||
        RegExp(r'^[A-Za-z]:/').hasMatch(normalized);
    if (looksAbsolute) return File(normalized);
    final root =
        recordsDir.path.replaceAll('\\', '/').replaceFirst(RegExp(r'/+$'), '');
    return File('$root/$normalized');
  }

  Map<String, Object?>? _readJsonMapOrNull(File file) {
    if (!file.existsSync()) return null;
    final decoded = jsonDecode(file.readAsStringSync());
    if (decoded is Map<String, Object?>) return decoded;
    if (decoded is Map) return Map<String, Object?>.from(decoded);
    return null;
  }

  /// Atomic-ish JSON write: tmp file, flush, replace. The delete-before-
  /// rename window is the documented cross-platform tradeoff (Windows
  /// rename-over-exists); the registry philosophy (files are truth, index
  /// is rebuildable) bounds the worst case to a rebuild.
  void _atomicWriteJson(File target, Map<String, Object?> doc) {
    final encoder = const JsonEncoder.withIndent('  ');
    final tmp = File('${target.path}.tmp');
    tmp.writeAsStringSync('${encoder.convert(doc)}\n', flush: true);
    if (target.existsSync()) target.deleteSync();
    tmp.renameSync(target.path);
  }

  /// Sanitizes a free-text query into a safe FTS5 MATCH expression:
  /// tokens are stripped of match-syntax characters, quoted, ANDed.
  String? _ftsMatchQuery(String query) {
    final tokens = query
        .split(RegExp(r'\s+'))
        .map((t) => t.replaceAll(RegExp(r'["\*(){}:^\-]'), '').trim())
        .where((t) => t.isNotEmpty)
        .toSet()
        .toList();
    if (tokens.isEmpty) return null;
    return tokens.map((t) => '"$t"').join(' AND ');
  }
}

/// Result of write path 1.
class SessionAppendReport {
  const SessionAppendReport({
    required this.sessionId,
    required this.mintedRecordIds,
    required this.mintedPaths,
    required this.sessionFile,
  });

  final String sessionId;
  final List<String> mintedRecordIds;
  final List<String> mintedPaths;
  final String sessionFile;
}

/// Result of write path 2. [accepted] is the whole story: denied attempts
/// never touch files or rows, only gate_log.
class GateOutcome {
  const GateOutcome.accepted({
    required this.recordId,
    required this.action,
    required this.actor,
    required this.ts,
  })  : accepted = true,
        reason = null;

  const GateOutcome.denied({
    required this.recordId,
    required this.action,
    required this.actor,
    required this.reason,
    required this.ts,
  }) : accepted = false;

  final bool accepted;
  final String recordId;
  final GateAction action;
  final String actor;
  final String? reason;
  final String ts;
}

/// Result of write path 3. [gateLogPreserved] is the contract check: the
/// stamp album survives every rebuild byte-for-byte in row count.
class RebuildReport {
  const RebuildReport({
    required this.records,
    required this.sessions,
    required this.gateLogPreserved,
    required this.at,
  });

  final int records;
  final int sessions;
  final int gateLogPreserved;
  final String at;
}

/// One gate_log stamp (or denied attempt), for the 5-b-4 viewer.
class GateLogRow {
  const GateLogRow({
    required this.ts,
    required this.recordId,
    required this.action,
    required this.actor,
    required this.note,
  });

  final String ts;
  final String recordId;
  final String action;
  final String actor;
  final String note;
}

/// One FTS hit.
class RegistrySearchHit {
  const RegistrySearchHit({
    required this.recordId,
    required this.recordType,
    required this.status,
    required this.canonicalText,
    required this.confidence,
    required this.rank,
  });

  final String recordId;
  final String recordType;
  final String status;
  final String canonicalText;
  final double confidence;
  final double rank;
}

/// Native library bootstrap for hosts where the sqlite3 package's default
/// loader name is not on the path (Ubuntu ships `libsqlite3.so.0`; a
/// Windows Bench machine may have no sqlite3 anywhere findable at all).
/// Call once from the app or test bootstrap BEFORE opening a registry.
/// Idempotent.
///
/// Search order, first hit wins:
///   1. the `SAM_SQLITE3_PATH` environment variable (absolute path — the
///      Bench app and power users can pin an exact dll/so);
///   2. the bare platform name (`sqlite3.dll` on Windows, `libsqlite3.so.0`
///      on Linux, `libsqlite3.dylib` on macOS) — covers system dirs, PATH,
///      and the dynamic loader's own search;
///   3. the same names beside the current directory and inside `test/`
///      ("drop the dll next to pubspec.yaml" just works for flutter test);
///   4. the other platforms' bare names (cross-platform tolerance).
///
/// If nothing loads, throws an [ArgumentError] listing every candidate
/// tried plus the exact fix — a bare "error 126" with a Linux name on a
/// Windows machine (the 5-b-2 first Windows run, 2026-09-16) is not an
/// acceptable diagnostic for Dave.
void registerSqliteOverride() {
  sqlite_open.open.overrideForAll(() {
    final tried = <String>[];

    String? pinned = Platform.environment['SAM_SQLITE3_PATH'];
    if (pinned != null && pinned.trim().isNotEmpty) {
      pinned = pinned.trim();
      tried.add(pinned);
      try {
        return DynamicLibrary.open(pinned);
      } catch (_) {}
    }

    final String primary;
    if (Platform.isWindows) {
      primary = 'sqlite3.dll';
    } else if (Platform.isMacOS) {
      primary = 'libsqlite3.dylib';
    } else {
      primary = 'libsqlite3.so.0';
    }
    final names = <String>{
      primary,
      'libsqlite3.so.0',
      'libsqlite3.so',
      'libsqlite3.dylib',
      'sqlite3.dll',
    }.toList();

    final sep = Platform.isWindows ? r'\' : '/';
    final cwd = Directory.current.path;
    for (final name in names) {
      // (a) bare name: system dirs, PATH, loader defaults.
      tried.add(name);
      try {
        return DynamicLibrary.open(name);
      } catch (_) {}
      // (b) beside the current directory (next to pubspec.yaml).
      final beside = '$cwd$sep$name';
      tried.add(beside);
      try {
        return DynamicLibrary.open(beside);
      } catch (_) {}
      // (c) inside test/ (flutter test from the package root).
      final inTest = '$cwd${sep}test$sep$name';
      tried.add(inTest);
      try {
        return DynamicLibrary.open(inTest);
      } catch (_) {}
    }

    throw ArgumentError(
      'Could not load the SQLite native library. Tried:\n'
      '  ${tried.join('\n  ')}\n'
      '\n'
      'Fix (Windows): download "sqlite-dll-win-x64-*.zip" from '
      'https://www.sqlite.org/download.html, unzip it, and copy '
      'sqlite3.dll into the calli-archiviste package folder (next to '
      'pubspec.yaml) or anywhere on PATH.\n'
      'Fix (Linux): install libsqlite3 (e.g. sudo apt install '
      'libsqlite3-dev) or place libsqlite3.so.0 on the loader path.\n'
      'Alternative: set the SAM_SQLITE3_PATH environment variable to the '
      'full path of the library file and re-run.',
    );
  });
}
