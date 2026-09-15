import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:calli_archiviste/config/pkc_config.dart';
import 'package:calli_archiviste/models/pkc_interview_evidence.dart';

abstract interface class PKCInterviewBridge {
  Future<PKCInterviewEvidence> retrieveEvidence(PKCInterviewRequest request);
}

class PKCBridgeProcessResult {
  const PKCBridgeProcessResult({
    required this.exitCode,
    required this.stdout,
    required this.stderr,
    this.timedOut = false,
  });

  final int exitCode;
  final String stdout;
  final String stderr;
  final bool timedOut;
}

abstract interface class PKCBridgeProcessRunner {
  Future<PKCBridgeProcessResult> run({
    required String executable,
    required List<String> arguments,
    required String input,
    required String workingDirectory,
    required Duration timeout,
  });
}

class SystemPKCBridgeProcessRunner implements PKCBridgeProcessRunner {
  const SystemPKCBridgeProcessRunner();

  @override
  Future<PKCBridgeProcessResult> run({
    required String executable,
    required List<String> arguments,
    required String input,
    required String workingDirectory,
    required Duration timeout,
  }) async {
    final process = await Process.start(
      executable,
      arguments,
      workingDirectory: workingDirectory,
      runInShell: false,
    );
    final stdoutFuture = process.stdout.transform(utf8.decoder).join();
    final stderrFuture = process.stderr.transform(utf8.decoder).join();
    process.stdin.write(input);
    await process.stdin.close();

    try {
      final values = await Future.wait<Object>([
        process.exitCode,
        stdoutFuture,
        stderrFuture,
      ]).timeout(timeout);
      return PKCBridgeProcessResult(
        exitCode: values[0] as int,
        stdout: values[1] as String,
        stderr: values[2] as String,
      );
    } on TimeoutException {
      process.kill();
      return const PKCBridgeProcessResult(
        exitCode: -1,
        stdout: '',
        stderr: '',
        timedOut: true,
      );
    }
  }
}

class LocalProcessPKCInterviewBridge implements PKCInterviewBridge {
  LocalProcessPKCInterviewBridge({
    required this.enabled,
    required this.pythonExecutable,
    required this.bridgeScript,
    this.pkcRoot,
    this.timeout = const Duration(seconds: 20),
    PKCBridgeProcessRunner processRunner = const SystemPKCBridgeProcessRunner(),
  }) : _processRunner = processRunner;

  /// Donor-compatible zero-argument factory: reads the global PKCConfig
  /// (dotenv-driven; disabled unless deliberately enabled).
  factory LocalProcessPKCInterviewBridge.fromConfig() =>
      LocalProcessPKCInterviewBridge(
        enabled: PKCConfig.enabled,
        pythonExecutable: PKCConfig.pythonExecutable,
        bridgeScript: PKCConfig.bridgeScript,
        pkcRoot: PKCConfig.pkcRoot,
        timeout: PKCConfig.timeout,
      );

  final bool enabled;
  final String pythonExecutable;
  final String bridgeScript;
  final String? pkcRoot;
  final Duration timeout;
  final PKCBridgeProcessRunner _processRunner;

  @override
  Future<PKCInterviewEvidence> retrieveEvidence(
    PKCInterviewRequest request,
  ) async {
    if (!enabled) {
      return PKCInterviewEvidence.unavailable('disabled');
    }

    final script = File(bridgeScript);
    if (!script.isAbsolute || !await script.exists()) {
      return PKCInterviewEvidence.unavailable('bridge_unavailable');
    }

    final arguments = <String>[
      script.path,
      '--timeout-seconds',
      timeout.inSeconds.toString(),
      if (pkcRoot != null) ...['--pkc-root', pkcRoot!],
    ];

    try {
      final result = await _processRunner.run(
        executable: pythonExecutable,
        arguments: arguments,
        input: jsonEncode(request.toJson()),
        workingDirectory: script.parent.path,
        timeout: timeout + const Duration(seconds: 2),
      );
      if (result.timedOut) {
        return PKCInterviewEvidence.unavailable('timeout');
      }
      if (result.exitCode != 0) {
        return PKCInterviewEvidence.unavailable('bridge_process_failed');
      }
      final decoded = jsonDecode(result.stdout);
      if (decoded is! Map) {
        return PKCInterviewEvidence.unavailable('malformed_response');
      }
      return PKCInterviewEvidence.fromBridgeJson(
        Map<String, dynamic>.from(decoded),
      );
    } on FormatException {
      return PKCInterviewEvidence.unavailable('malformed_response');
    } on Object {
      return PKCInterviewEvidence.unavailable('pkc_unavailable');
    }
  }
}