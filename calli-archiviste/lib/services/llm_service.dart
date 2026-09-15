/// Shared interface for the host application's configured cloud or local LLM backend.
abstract class LlmService {
  Future<String> sendMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    int? maxTokens,
    String? model,
  });

  Stream<String> streamMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    int? maxTokens,
    String? model,
  });
}
