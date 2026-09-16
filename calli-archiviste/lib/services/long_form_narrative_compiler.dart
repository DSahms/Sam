import 'package:flutter/services.dart';
import 'package:calli_archiviste/config/api_config.dart';
import 'package:calli_archiviste/config/chapter_catalog.dart';
import 'package:calli_archiviste/config/constants.dart';
import 'package:calli_archiviste/models/interview_session.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/services/llm_service.dart';

/// Staged compiler for long interview sessions that exceed one model context/output window.
///
/// Extracted from the donor application (STAGE 4). The donor used the same
/// chunk -> assemble pipeline against its own assets/prompts files; Calli
/// ships identical prompt texts under assets/prompts/.
class LongFormNarrativeCompiler {
  final LlmService _llm;

  /// User answers per compilation chunk (each chunk also includes its questions).
  static const int answersPerChunk = 12;

  /// Output token budget per chunk (substantive prose, not summary).
  static const int chunkMaxTokens = 4096;

  /// Output token budget for final assembly pass.
  static const int assembleMaxTokens = 8192;

  LongFormNarrativeCompiler({required LlmService llm}) : _llm = llm;

  Future<String> compileChapter({
    required LifeChapter chapter,
    required List<InterviewSession> sessions,
    required String userName,
  }) async {
    final sorted = List<InterviewSession>.from(sessions)
      ..sort((a, b) => a.startedAt.compareTo(b.startedAt));

    final pairs = <_QaPair>[];
    for (final session in sorted) {
      pairs.addAll(_extractPairs(session, userName));
    }
    if (pairs.isEmpty) {
      return '';
    }

    if (pairs.length <= answersPerChunk) {
      return _compileSinglePass(
        chapter: chapter,
        pairs: pairs,
        userName: userName,
      );
    }

    final chunks = _chunkPairs(pairs);
    final chunkNarratives = <String>[];
    for (var i = 0; i < chunks.length; i++) {
      final section = await _compileChunk(
        chapter: chapter,
        pairs: chunks[i],
        userName: userName,
        chunkNumber: i + 1,
        chunkTotal: chunks.length,
      );
      if (section.trim().isNotEmpty) {
        chunkNarratives.add(section.trim());
      }
    }

    if (chunkNarratives.isEmpty) {
      return '';
    }
    if (chunkNarratives.length == 1) {
      return chunkNarratives.first;
    }

    return _assembleSections(
      chapter: chapter,
      sections: chunkNarratives,
      userName: userName,
    );
  }

  List<_QaPair> _extractPairs(InterviewSession session, String userName) {
    final pairs = <_QaPair>[];
    String? pendingQuestion;
    for (final message in session.messages) {
      if (message.role == MessageRole.interviewer) {
        pendingQuestion = message.content;
      } else if (message.role == MessageRole.user && pendingQuestion != null) {
        pairs.add(
          _QaPair(
            question: pendingQuestion,
            answer: message.content,
          ),
        );
        pendingQuestion = null;
      }
    }
    return pairs;
  }

  List<List<_QaPair>> _chunkPairs(List<_QaPair> pairs) {
    final chunks = <List<_QaPair>>[];
    for (var i = 0; i < pairs.length; i += answersPerChunk) {
      final end = (i + answersPerChunk > pairs.length)
          ? pairs.length
          : i + answersPerChunk;
      chunks.add(pairs.sublist(i, end));
    }
    return chunks;
  }

  String _formatPairs(List<_QaPair> pairs, String userName) {
    final buffer = StringBuffer();
    for (final pair in pairs) {
      buffer.writeln('Interviewer: ${pair.question}');
      buffer.writeln('$userName: ${pair.answer}');
      buffer.writeln();
    }
    return buffer.toString();
  }

  Future<String> _compileSinglePass({
    required LifeChapter chapter,
    required List<_QaPair> pairs,
    required String userName,
  }) async {
    var prompt = await rootBundle.loadString(
      'packages/calli_archiviste/assets/prompts/narrative_chunk_system.txt',
    );
    final chapterName = ChapterCatalog.title(chapter);
    prompt = prompt
        .replaceAll('{{chapter_name}}', chapterName)
        .replaceAll('{{user_name}}', userName)
        .replaceAll('{{chunk_number}}', '1')
        .replaceAll('{{chunk_total}}', '1')
        .replaceAll('{{transcripts}}', _formatPairs(pairs, userName));

    return _llm.sendMessage(
      systemPrompt: prompt,
      messages: const [
        {
          'role': 'user',
          'content':
              'Convert this interview chunk into faithful first-person memoir prose.',
        },
      ],
      maxTokens: chunkMaxTokens,
      model: ApiConfig.narrativeModel,
    );
  }

  Future<String> _compileChunk({
    required LifeChapter chapter,
    required List<_QaPair> pairs,
    required String userName,
    required int chunkNumber,
    required int chunkTotal,
  }) async {
    var prompt = await rootBundle.loadString(
      'packages/calli_archiviste/assets/prompts/narrative_chunk_system.txt',
    );
    final chapterName = ChapterCatalog.title(chapter);
    prompt = prompt
        .replaceAll('{{chapter_name}}', chapterName)
        .replaceAll('{{user_name}}', userName)
        .replaceAll('{{chunk_number}}', '$chunkNumber')
        .replaceAll('{{chunk_total}}', '$chunkTotal')
        .replaceAll('{{transcripts}}', _formatPairs(pairs, userName));

    return _llm.sendMessage(
      systemPrompt: prompt,
      messages: const [
        {
          'role': 'user',
          'content':
              'Convert this interview chunk into faithful first-person memoir prose.',
        },
      ],
      maxTokens: chunkMaxTokens,
      model: ApiConfig.narrativeModel,
    );
  }

  Future<String> _assembleSections({
    required LifeChapter chapter,
    required List<String> sections,
    required String userName,
  }) async {
    var prompt = await rootBundle.loadString(
      'packages/calli_archiviste/assets/prompts/narrative_assemble_system.txt',
    );
    final chapterName = ChapterCatalog.title(chapter);
    final body = StringBuffer();
    for (var i = 0; i < sections.length; i++) {
      body.writeln('--- Section ${i + 1} ---');
      body.writeln(sections[i]);
      body.writeln();
    }
    prompt = prompt
        .replaceAll('{{chapter_name}}', chapterName)
        .replaceAll('{{user_name}}', userName)
        .replaceAll('{{sections}}', body.toString());

    return _llm.sendMessage(
      systemPrompt: prompt,
      messages: const [
        {
          'role': 'user',
          'content': 'Assemble these sections into one continuous chapter narrative.',
        },
      ],
      maxTokens: assembleMaxTokens,
      model: ApiConfig.narrativeModel,
    );
  }
}

class _QaPair {
  final String question;
  final String answer;

  const _QaPair({required this.question, required this.answer});
}

/// Forensic helpers for analyzing stored sessions without modifying them.
///
/// Extracted alongside the compiler because the donor kept them in the same
/// file; the donor's session export tooling consumes these helpers and may be
/// extracted later.
class SessionForensics {
  static InterviewSession? largestSession(List<InterviewSession> sessions) {
    if (sessions.isEmpty) return null;
    return sessions.reduce(
      (a, b) => a.messages.length >= b.messages.length ? a : b,
    );
  }

  static int interviewerCount(InterviewSession session) {
    return session.messages
        .where((m) => m.role == MessageRole.interviewer)
        .length;
  }

  static int userAnswerCount(InterviewSession session) {
    return session.answerCount;
  }

  static int uniqueQuestionCount(InterviewSession session) {
    final seen = <String>{};
    for (final m in session.messages) {
      if (m.role == MessageRole.interviewer) {
        seen.add(m.content.trim().toLowerCase());
      }
    }
    return seen.length;
  }

  static double repetitionRatio(InterviewSession session) {
    final questions = session.messages
        .where((m) => m.role == MessageRole.interviewer)
        .map((m) => m.content.trim().toLowerCase())
        .toList();
    if (questions.isEmpty) return 0;
    final unique = questions.toSet().length;
    return 1 - (unique / questions.length);
  }
}
