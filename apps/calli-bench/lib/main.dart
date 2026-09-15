import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:hive/hive.dart';
import 'package:path_provider/path_provider.dart';

import 'package:calli_archiviste/calli_archiviste.dart';

/// User name injected into engine, compressor and narrative prompts.
/// BENCH DEFAULT: persona/naming rules for the product UI come with the
/// real Sam host (Stage 5-b). The bench hardcodes the owner's name.
const String kUserName = 'Dave';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Personal interview content must never live inside the repository
  // (migration anti-pattern 3: no personal data in Git). Hive boxes go to
  // the user's documents directory instead.
  final docs = await getApplicationDocumentsDirectory();
  final dataDir = Directory(
    '${docs.path}${Platform.pathSeparator}calli_bench_data',
  );
  await dataDir.create(recursive: true);
  Hive.init(dataDir.path);

  // The reserved loopback endpoint (127.0.0.1:5001) is enforced by
  // KoboldLocalService itself; LocalModelConfig reads optional .env
  // overrides but defaults to the KoboldCpp server on this machine.
  final llm = KoboldLocalService.fromConfig();

  final storage = SessionStorage();
  await storage.initialize();
  final intakeStorage = IntakeStorage();
  await intakeStorage.initialize();

  final compressor = ContextCompressor(llm: llm, userName: kUserName);
  await compressor.initialize();

  final engine = InterviewEngine(
    llm: llm,
    contextCompressor: compressor,
    storage: storage,
    // The bench runs without the PKC bridge: live question enrichment is
    // disabled, matching the engine's production default.
    pkcRetrievalEnabled: false,
    pkcModelEnrichmentEnabled: false,
    userName: kUserName,
  );
  await engine.initialize();

  final provider = InterviewProvider(
    engine: engine,
    storage: storage,
    intakeStorage: intakeStorage,
  );

  runApp(
    CalliBenchApp(
      provider: provider,
      storage: storage,
      narrative: NarrativeService(llm: llm),
      kobold: llm,
    ),
  );
}

class CalliBenchApp extends StatelessWidget {
  const CalliBenchApp({
    super.key,
    required this.provider,
    required this.storage,
    required this.narrative,
    required this.kobold,
  });

  final InterviewProvider provider;
  final SessionStorage storage;
  final NarrativeService narrative;
  final KoboldLocalService kobold;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Calli Bench',
      theme: ThemeData(
        colorSchemeSeed: const Color(0xFF5B7C99),
        useMaterial3: true,
      ),
      home: BenchScreen(
        provider: provider,
        storage: storage,
        narrative: narrative,
        kobold: kobold,
      ),
    );
  }
}

class BenchScreen extends StatefulWidget {
  const BenchScreen({
    super.key,
    required this.provider,
    required this.storage,
    required this.narrative,
    required this.kobold,
  });

  final InterviewProvider provider;
  final SessionStorage storage;
  final NarrativeService narrative;
  final KoboldLocalService kobold;

  @override
  State<BenchScreen> createState() => _BenchScreenState();
}

class _BenchScreenState extends State<BenchScreen> {
  LifeChapter _chapter = LifeChapter.childhood;
  final TextEditingController _answerController = TextEditingController();
  String _koboldStatus = 'Checking Kobold...';
  bool _koboldOk = false;
  bool _memoirBusy = false;

  @override
  void initState() {
    super.initState();
    _checkKobold();
  }

  @override
  void dispose() {
    _answerController.dispose();
    super.dispose();
  }

  Future<void> _checkKobold() async {
    setState(() => _koboldStatus = 'Checking Kobold...');
    try {
      final id = await widget.kobold.loadedModelId();
      setState(() {
        _koboldOk = true;
        _koboldStatus = 'Kobold OK: $id @ ${LocalModelConfig.endpoint}';
      });
    } catch (e) {
      setState(() {
        _koboldOk = false;
        _koboldStatus =
            'Kobold not reachable at ${LocalModelConfig.endpoint} - '
            'start KoboldCpp, then tap refresh';
      });
    }
  }

  Future<void> _sendAnswer() async {
    final text = _answerController.text.trim();
    if (text.isEmpty) return;
    _answerController.clear();
    await widget.provider.sendMessage(text);
  }

  Future<void> _writeMemoir() async {
    if (_memoirBusy) return;
    setState(() => _memoirBusy = true);
    final messenger = ScaffoldMessenger.of(context);
    try {
      final all =
          await widget.storage.getSessionsForChapter(_chapter);
      final usable = all
          .where((s) => s.isComplete && s.answerCount > 0)
          .toList();
      if (!mounted) return;
      if (usable.isEmpty) {
        messenger.showSnackBar(
          SnackBar(
            content: Text(
              'No completed sessions for "${ChapterCatalog.title(_chapter)}" '
              'yet. End a session first, then write the memoir.',
            ),
          ),
        );
        return;
      }
      final text = await widget.narrative.generateChapterNarrative(
        chapter: _chapter,
        sessions: usable,
        userName: kUserName,
      );
      final signature =
          NarrativeService.narrativeSignatureForSessions(usable);
      await widget.storage.saveNarrative(
        _chapter,
        text,
        sourceSignature: signature,
      );
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (dialogContext) => AlertDialog(
          title: Text('Memoir - ${ChapterCatalog.title(_chapter)}'),
          content: SizedBox(
            width: 640,
            height: 420,
            child: SingleChildScrollView(
              child: SelectableText(
                text.isEmpty ? '(The model returned nothing.)' : text,
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: text));
                messenger.showSnackBar(
                  const SnackBar(
                    content: Text('Memoir copied to the clipboard.'),
                  ),
                );
              },
              child: const Text('Copy all'),
            ),
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    } catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text('Memoir failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _memoirBusy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(
          'Calli Bench - real engine on Kobold '
          '(${LocalModelConfig.endpoint})',
        ),
        actions: [
          IconButton(
            onPressed: _checkKobold,
            tooltip: 'Re-check Kobold',
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: AnimatedBuilder(
        animation: widget.provider,
        builder: (context, _) {
          final provider = widget.provider;
          final session = provider.currentSession;
          return Column(
            children: [
              _HeaderBar(
                chapter: _chapter,
                onChapterChanged: provider.isGenerating
                    // Explicit non-nullable parameter: the header field is
                    // ValueChanged<LifeChapter?>? to match DropdownButton's
                    // onChanged, so an untyped lambda would infer LifeChapter?
                    // and fail to assign _chapter. The dropdown's value and
                    // items are always non-null, so onChanged never fires
                    // with null; a Function(LifeChapter) is assignable to
                    // Function(LifeChapter?) by contravariance.
                    ? null
                    : (LifeChapter value) =>
                        setState(() => _chapter = value),
                hasSession: session != null,
                isGenerating: provider.isGenerating,
                onStart: () =>
                    provider.startNewSession(_chapter),
                onEnd: () => provider.endSession(),
                onMemoir: _writeMemoir,
                memoirBusy: _memoirBusy,
                koboldStatus: _koboldStatus,
                koboldOk: _koboldOk,
                answerCount: session?.answerCount ?? 0,
              ),
              if (provider.error != null)
                Material(
                  color: Colors.red.shade900,
                  child: ListTile(
                    dense: true,
                    leading:
                        const Icon(Icons.error_outline, color: Colors.white),
                    title: Text(
                      provider.error!,
                      style: const TextStyle(color: Colors.white),
                    ),
                    trailing: IconButton(
                      icon: const Icon(Icons.close, color: Colors.white),
                      onPressed: provider.clearError,
                    ),
                  ),
                ),
              if (provider.isGenerating)
                const LinearProgressIndicator(minHeight: 3),
              Expanded(
                child: session == null
                    ? const _EmptyState()
                    : _Transcript(messages: session.messages),
              ),
              if (provider.isGenerating)
                const Padding(
                  padding: EdgeInsets.all(8),
                  child: Text('Kobold is thinking... '
                      '(probe-deeper and compression traces appear in the '
                      'run console)'),
                ),
              _AnswerBar(
                controller: _answerController,
                enabled: session != null && !provider.isGenerating,
                onSend: _sendAnswer,
              ),
            ],
          );
        },
      ),
    );
  }
}

class _HeaderBar extends StatelessWidget {
  const _HeaderBar({
    required this.chapter,
    required this.onChapterChanged,
    required this.hasSession,
    required this.isGenerating,
    required this.onStart,
    required this.onEnd,
    required this.onMemoir,
    required this.memoirBusy,
    required this.koboldStatus,
    required this.koboldOk,
    required this.answerCount,
  });

  final LifeChapter chapter;
  final ValueChanged<LifeChapter?>? onChapterChanged;
  final bool hasSession;
  final bool isGenerating;
  final VoidCallback onStart;
  final VoidCallback onEnd;
  final VoidCallback onMemoir;
  final bool memoirBusy;
  final String koboldStatus;
  final bool koboldOk;
  final int answerCount;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.fromLTRB(12, 12, 12, 6),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: DropdownButton<LifeChapter>(
                    value: chapter,
                    isExpanded: true,
                    onChanged: onChapterChanged,
                    items: [
                      for (final value in LifeChapter.values)
                        DropdownMenuItem(
                          value: value,
                          child: Text(ChapterCatalog.title(value)),
                        ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  onPressed:
                      hasSession || isGenerating ? null : onStart,
                  child: const Text('New session'),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  onPressed:
                      hasSession && !isGenerating ? onEnd : null,
                  child: const Text('End session'),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: isGenerating || memoirBusy
                      ? null
                      : onMemoir,
                  child: memoirBusy
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('Write memoir'),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Row(
              children: [
                Icon(
                  koboldOk ? Icons.check_circle : Icons.error_outline,
                  size: 16,
                  color: koboldOk ? Colors.green.shade700 : Colors.red.shade700,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    koboldStatus,
                    style: Theme.of(context).textTheme.bodySmall,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                Text(
                  'Answers: $answerCount',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.record_voice_over,
              size: 44, color: Colors.grey.shade500),
          const SizedBox(height: 12),
          const Text(
            'Pick a chapter and press "New session".\n'
            'The engine opens with the chapter seed question.\n'
            'Dictate or type your answers - no keyboard required.',
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _Transcript extends StatelessWidget {
  const _Transcript({required this.messages});

  final List<Message> messages;

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      itemCount: messages.length,
      itemBuilder: (context, index) {
        final message = messages[index];
        final isInterviewer = message.role == MessageRole.interviewer;
        return Align(
          alignment: isInterviewer
              ? Alignment.centerLeft
              : Alignment.centerRight,
          child: Container(
            margin: const EdgeInsets.symmetric(vertical: 4),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            constraints: const BoxConstraints(maxWidth: 560),
            decoration: BoxDecoration(
              color: isInterviewer ? Colors.blueGrey.shade50 : Colors.teal.shade50,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: isInterviewer
                    ? Colors.blueGrey.shade200
                    : Colors.teal.shade200,
              ),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  isInterviewer ? 'Calli' : kUserName,
                  style: Theme.of(context).textTheme.labelSmall,
                ),
                const SizedBox(height: 2),
                SelectableText(message.content),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _AnswerBar extends StatelessWidget {
  const _AnswerBar({
    required this.controller,
    required this.enabled,
    required this.onSend,
  });

  final TextEditingController controller;
  final bool enabled;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 4, 12, 12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                enabled: enabled,
                minLines: 1,
                maxLines: 4,
                decoration: const InputDecoration(
                  hintText:
                      'Your answer (voice-to-text works in this field)...',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ),
            const SizedBox(width: 8),
            FilledButton.icon(
              onPressed: enabled ? onSend : null,
              icon: const Icon(Icons.send),
              label: const Text('Send'),
            ),
          ],
        ),
      ),
    );
  }
}
