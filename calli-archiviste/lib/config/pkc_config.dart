import 'dart:io';

import 'package:flutter_dotenv/flutter_dotenv.dart';

/// Local-only PKC bridge configuration. Disabled unless deliberately enabled.
class PKCConfig {

  // Extraction guard (2026-09-15): the host app may not have loaded a .env
  // yet; missing configuration then falls back to the getter defaults
  // instead of throwing.
  static String? _env(String key) {
    try {
      return dotenv.env[key];
    } catch (_) {
      return null;
    }
  }

  static bool get enabled =>
      _env('PKC_BRIDGE_ENABLED')?.trim().toLowerCase() == 'true';

  static bool get modelEnrichmentEnabled =>
      _env('PKC_MODEL_ENRICHMENT_ENABLED')?.trim().toLowerCase() ==
      'true';

  static String? get enrichmentSourceId {
    final configured = _env('PKC_ENRICHMENT_SOURCE_ID')?.trim();
    return configured == null || configured.isEmpty ? null : configured;
  }

  static String get pythonExecutable {
    final configured = _env('PKC_PYTHON_EXECUTABLE')?.trim();
    return configured == null || configured.isEmpty ? 'python' : configured;
  }

  static String get bridgeScript {
    final configured = _env('PKC_BRIDGE_SCRIPT')?.trim();
    if (configured != null && configured.isNotEmpty) {
      return configured;
    }
    return [
      Directory.current.path,
      'tools',
      'storykeeper_pkc_bridge.py',
    ].join(Platform.pathSeparator);
  }

  static String? get pkcRoot {
    final configured = _env('PKC_ROOT')?.trim();
    return configured == null || configured.isEmpty ? null : configured;
  }

  static Duration get timeout {
    final seconds =
        int.tryParse(_env('PKC_BRIDGE_TIMEOUT_SECONDS')?.trim() ?? '') ??
        20;
    return Duration(seconds: seconds.clamp(1, 120).toInt());
  }
}
