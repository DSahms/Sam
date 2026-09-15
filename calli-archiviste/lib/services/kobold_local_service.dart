import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:calli_archiviste/config/local_model_config.dart';
import 'package:calli_archiviste/services/llm_service.dart';

/// OpenAI-compatible local-only client for the existing KoboldCpp server.
class KoboldLocalService implements LlmService {
  KoboldLocalService({
    required String endpoint,
    required this.defaultModel,
    this.timeout = const Duration(seconds: 90),
    http.Client? client,
  }) : _endpoint = _validatedEndpoint(endpoint),
       _client = client ?? http.Client();

  factory KoboldLocalService.fromConfig() => KoboldLocalService(
    endpoint: LocalModelConfig.endpoint,
    defaultModel: LocalModelConfig.model,
    timeout: LocalModelConfig.timeout,
  );

  final Uri _endpoint;
  final String defaultModel;
  final Duration timeout;
  final http.Client _client;

  Duration? lastRequestLatency;

  static Uri _validatedEndpoint(String value) {
    final uri = Uri.parse(value);
    if (uri.scheme != 'http' ||
        uri.host != '127.0.0.1' ||
        !uri.hasPort ||
        uri.port != 5001) {
      throw ArgumentError(
        'The local model must use the reserved loopback endpoint.',
      );
    }
    return uri;
  }

  Uri _apiUri(String path) {
    final base = _endpoint.toString().replaceFirst(RegExp(r'/$'), '');
    return Uri.parse('$base/v1/$path');
  }

  Future<String> loadedModelId() async {
    final response = await _client.get(_apiUri('models')).timeout(timeout);
    if (response.statusCode != 200) {
      throw StateError('Local model metadata request failed.');
    }
    final body = jsonDecode(response.body);
    if (body is! Map ||
        body['data'] is! List ||
        (body['data'] as List).isEmpty) {
      throw const FormatException('Local model metadata was malformed.');
    }
    final first = (body['data'] as List).first;
    if (first is! Map || first['id'] is! String) {
      throw const FormatException('Local model ID was unavailable.');
    }
    return first['id'] as String;
  }

  @override
  Future<String> sendMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    int? maxTokens,
    String? model,
  }) async {
    final stopwatch = Stopwatch()..start();
    try {
      final response = await _client
          .post(
            _apiUri('chat/completions'),
            headers: const {'Content-Type': 'application/json'},
            body: jsonEncode({
              'model': model ?? defaultModel,
              'messages': [
                {'role': 'system', 'content': systemPrompt},
                ...messages,
              ],
              'max_tokens': maxTokens ?? 512,
              'temperature': 0.2,
            }),
          )
          .timeout(timeout);
      if (response.statusCode != 200) {
        throw StateError('Local model request failed.');
      }
      final body = jsonDecode(response.body);
      if (body is! Map || body['choices'] is! List) {
        throw const FormatException('Local model response was malformed.');
      }
      final choices = body['choices'] as List;
      if (choices.isEmpty || choices.first is! Map) {
        throw const FormatException('Local model returned no choices.');
      }
      final message = (choices.first as Map)['message'];
      if (message is! Map || message['content'] is! String) {
        throw const FormatException('Local model returned no content.');
      }
      debugPrint(
        'LOCAL MODEL: endpoint=$_endpoint '
        'model=${model ?? defaultModel} '
        'latencyMs=${stopwatch.elapsedMilliseconds}',
      );
      return (message['content'] as String).trim();
    } finally {
      stopwatch.stop();
      lastRequestLatency = stopwatch.elapsed;
    }
  }

  @override
  Stream<String> streamMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    int? maxTokens,
    String? model,
  }) async* {
    final response = await sendMessage(
      systemPrompt: systemPrompt,
      messages: messages,
      maxTokens: maxTokens,
      model: model,
    );
    yield response;
  }
}
