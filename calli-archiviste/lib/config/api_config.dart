import 'package:flutter_dotenv/flutter_dotenv.dart';

enum LlmBackend { anthropic, groq, venice }

/// API configuration loaded from .env file
class ApiConfig {

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

  static LlmBackend get backend {
    final raw = _env('LLM_BACKEND')?.trim().toLowerCase();
    if (raw == 'groq') {
      return LlmBackend.groq;
    }
    if (raw == 'venice') {
      return LlmBackend.venice;
    }
    return LlmBackend.anthropic;
  }

  static String get backendLabel {
    switch (backend) {
      case LlmBackend.groq:
        return 'Groq (Llama)';
      case LlmBackend.venice:
        return 'Venice (Llama)';
      case LlmBackend.anthropic:
        return 'Anthropic (Claude)';
    }
  }

  static String get apiKey {
    final key = _env('ANTHROPIC_API_KEY');
    if (key == null || key.isEmpty) {
      throw Exception('ANTHROPIC_API_KEY not found in .env file');
    }
    return key;
  }

  static String get groqApiKey {
    final key = _env('GROQ_API_KEY');
    if (key == null || key.isEmpty) {
      throw Exception('GROQ_API_KEY not found in .env file');
    }
    return key;
  }

  static String get veniceApiKey {
    final key = _env('VENICE_API_KEY');
    if (key == null || key.isEmpty) {
      throw Exception('VENICE_API_KEY not found in .env file');
    }
    return key;
  }

  static const String baseUrl = 'https://api.anthropic.com/v1/messages';
  static const String groqBaseUrl =
      'https://api.groq.com/openai/v1/chat/completions';
  static const String veniceBaseUrl =
      'https://api.venice.ai/api/v1/chat/completions';
  static const String apiVersion = '2023-06-01';

  static const String defaultInterviewModel = 'claude-sonnet-4-20250514';
  static const String defaultNarrativeModel = 'claude-sonnet-4-20250514';
  static const String defaultGroqModel = 'llama-3.3-70b-versatile';
  static const String defaultVeniceModel = 'llama-3.3-70b';

  static String get interviewModel {
    if (backend == LlmBackend.groq) {
      return _envModel('GROQ_MODEL') ?? defaultGroqModel;
    }
    if (backend == LlmBackend.venice) {
      return _envModel('VENICE_MODEL') ?? defaultVeniceModel;
    }
    return _envModel('ANTHROPIC_INTERVIEW_MODEL') ?? defaultInterviewModel;
  }

  static String get narrativeModel {
    if (backend == LlmBackend.groq) {
      return _envModel('GROQ_NARRATIVE_MODEL') ??
          _envModel('GROQ_MODEL') ??
          defaultGroqModel;
    }
    if (backend == LlmBackend.venice) {
      return _envModel('VENICE_NARRATIVE_MODEL') ??
          _envModel('VENICE_MODEL') ??
          defaultVeniceModel;
    }
    return _envModel('ANTHROPIC_NARRATIVE_MODEL') ?? defaultNarrativeModel;
  }

  static String get groqModel => _envModel('GROQ_MODEL') ?? defaultGroqModel;
  static String get veniceModel =>
      _envModel('VENICE_MODEL') ?? defaultVeniceModel;

  static String? _envModel(String key) {
    final value = dotenv.env[key]?.trim();
    if (value == null || value.isEmpty) {
      return null;
    }
    return value;
  }

  static const int defaultMaxTokens = 1024;
  static const int narrativeMaxTokens = 4096;
}
