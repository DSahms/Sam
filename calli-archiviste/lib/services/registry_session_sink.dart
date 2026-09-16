/// Bench-side bridge between [ChainSessionController] and [IntakeRegistry]
/// — Stage 5-b-3.
///
/// The controller stays sqlite-free (pure Dart, fake-able in tests); the
/// registry stays controller-free. This class is the one-way adapter the
/// Bench wires between them:
///
///   ChainSessionController -> RegistrySessionSink -> IntakeRegistry
///
/// The mapping is pass-through by design: [IntakeRegistry.sessionAppend]
/// takes exactly the four fields a [ChainSessionSink] receives, so this
/// class adds no logic of its own — no defaults, no swallowing. If the
/// registry throws (disk failure, invalid state), the error propagates to
/// the Bench UI: a failed write must be visible, never silently dropped
/// ("files are truth").
///
/// Approval semantics are unchanged here: landing files CANDIDATES only.
/// Nothing this class does can approve, reject, or close a record (I-4);
/// the gate is Stage 5-b-4 and Dave's alone.
library;

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/intake_registry.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';
import 'package:calli_archiviste/services/probe_session_controller.dart';

/// Files controller landings into the intake registry.
class RegistrySessionSink implements ChainSessionSink {
  RegistrySessionSink(this._registry);

  final IntakeRegistry _registry;

  @override
  void append({
    required ProbeChainState state,
    ClosureReport? closureReport,
    List<RecordCandidate> minted = const [],
    bool sessionClosed = false,
  }) {
    _registry.sessionAppend(
      state: state,
      closureReport: closureReport,
      minted: minted,
      sessionClosed: sessionClosed,
    );
  }
}
