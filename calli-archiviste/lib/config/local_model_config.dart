import 'package:flutter_dotenv/flutter_dotenv.dart';

/// Configuration for the local-only follow-up model.
class LocalModelConfig {

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

  static String get endpoint {
    final configured = _env('LOCAL_KOBOLD_ENDPOINT')?.trim();
    return configured == null || configured.isEmpty
        ? 'http://127.0.0.1:5001'
        : configured;
  }

  static String get model {
    final configured = _env('LOCAL_KOBOLD_MODEL')?.trim();
    return configured == null || configured.isEmpty ? 'koboldcpp' : configured;
  }

  static Duration get timeout {
    final seconds =
        int.tryParse(
          _env('LOCAL_KOBOLD_TIMEOUT_SECONDS')?.trim() ?? '',
        ) ??
        90;
    return Duration(seconds: seconds.clamp(1, 180).toInt());
  }
}
