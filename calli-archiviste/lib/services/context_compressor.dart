import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:calli_archiviste/models/message.dart';

/// Context compressor for summarizing interview sessions
/// Generates concise summaries to serve as context for future sessions
class ContextCompressor {
  final LlmService _llm;
  final String userName;
  late String _summarySystemPrompt;

  ContextCompressor({
    required LlmService llm,
    this.userName = '',
  }) : _llm = llm;

  /// Initialize the context compressor by loading system prompts
  Future<void> initialize() async {
    try {
      _summarySystemPrompt =
          await rootBundle.loadString('assets/prompts/context_summary_system.txt');
    } catch (e) {
      throw Exception('Failed to load context summary prompt: $e');
    }
  }

  /// Summarize a single session into concise context for future sessions
  /// Returns a summary string suitable for use as context in future questions
  Future<String> summarizeSession({
    required List<Message> messages,
    int maxTokens = 300,
  }) async {
    if (messages.isEmpty) {
      return '';
    }

    // Build conversation text from messages
    final conversationText = _buildConversationText(messages);

    try {
      final userMessage = userName.isNotEmpty
          ? 'The person being interviewed is named $userName. '
              'Use their name in the summary, not "the person".\n\n'
              'Here is a conversation from an intake interview session:\n\n$conversationText\n\nPlease summarize this conversation according to the guidelines provided.'
          : 'Here is a conversation from an intake interview session:\n\n$conversationText\n\nPlease summarize this conversation according to the guidelines provided.';

      final summary = await _llm.sendMessage(
        systemPrompt: _summarySystemPrompt,
        messages: [
          {
            'role': 'user',
            'content': userMessage,
          }
        ],
        maxTokens: maxTokens,
      );

      return summary;
    } catch (e) {
      debugPrint('ERROR: Session summary failed: $e');
      return 'Session covered ${messages.length} messages but summary generation failed.';
    }
  }

  /// Summarize multiple sessions into context
  /// Aggregates summaries from multiple sessions
  Future<String> summarizeSessions({
    required List<String> sessionSummaries,
    int maxTokens = 500,
  }) async {
    if (sessionSummaries.isEmpty) {
      return '';
    }

    // Combine all session summaries
    final combinedSummaries = sessionSummaries.join('\n\n---\n\n');

    try {
      final userMessage = userName.isNotEmpty
          ? 'The person being interviewed is named $userName. '
              'Use their name in the summary, not "the person".\n\n'
              'Here are summaries from multiple intake interview sessions:\n\n$combinedSummaries\n\nPlease create a unified summary that captures the key themes, relationships, and developments across all these sessions.'
          : 'Here are summaries from multiple intake interview sessions:\n\n$combinedSummaries\n\nPlease create a unified summary that captures the key themes, relationships, and developments across all these sessions.';

      final combinedSummary = await _llm.sendMessage(
        systemPrompt: _summarySystemPrompt,
        messages: [
          {
            'role': 'user',
            'content': userMessage,
          }
        ],
        maxTokens: maxTokens,
      );

      return combinedSummary;
    } catch (e) {
      // Return empty summary on error
      return '';
    }
  }

  /// Build conversation text from message history
  /// Formats messages for readability in summaries
  String _buildConversationText(List<Message> messages) {
    final buffer = StringBuffer();

    for (final message in messages) {
      final role = message.role == MessageRole.user ? 'Person' : 'Interviewer';
      buffer.writeln('$role: ${message.content}');
      buffer.writeln();
    }

    return buffer.toString();
  }
}

/// Abstract interface for LLM service
/// This allows Calli to use any LLM provider without coupling to a specific implementation
abstract class LlmService {
  Future<String> sendMessage({
    required String systemPrompt,
    required List<Map<String, String>> messages,
    required int maxTokens,
    String? model,
  });
}