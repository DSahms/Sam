import 'package:flutter/material.dart';

import 'package:calli_archiviste/calli_archiviste.dart';

/// The Stage 5-b-3 probe-chain screen: the Mother Leeds walk as a UI.
///
/// The screen is deliberately thin. It renders [ChainSessionController]
/// state and forwards intents (answer / skip / begin / land); every
/// decision lives in the engine, every wording decision is audited, and
/// nothing here can approve anything — the harvest panel files candidates
/// and says so.
///
/// One controller instance = one session. "New chain" builds a fresh
/// controller from [controllerFactory]; the old session's state stays in
/// the registry (files are truth), nothing is reused.
class ProbeChainScreen extends StatefulWidget {
  const ProbeChainScreen({
    super.key,
    required this.controllerFactory,
    this.registryError,
  });

  /// Builds a fresh controller per session (see the class doc for why).
  final ChainSessionController Function() controllerFactory;

  /// Non-null when the registry could not be opened (missing sqlite3.dll
  /// on Windows, path trouble): the screen explains instead of crashing
  /// the whole bench.
  final String? registryError;

  @override
  State<ProbeChainScreen> createState() => _ProbeChainScreenState();
}

class _ProbeChainScreenState extends State<ProbeChainScreen> {
  late ChainSessionController _controller;
  bool _busy = false;
  bool _landed = false;

  /// Set when [widget.controllerFactory] throws (registry unavailable).
  /// The error panel below renders this instead of crashing the tab —
  /// the interview bench keeps working regardless.
  String? _factoryError;

  final TextEditingController _topicController = TextEditingController();
  final TextEditingController _answerController = TextEditingController();
  final TextEditingController _skipReasonController =
      TextEditingController(text: "I don't remember");

  ElicitationMode _mode = ElicitationMode.freeRecall;

  void _setMode(ElicitationMode? value) {
    if (value == null || _busy) return;
    setState(() => _mode = value);
  }

  @override
  void initState() {
    super.initState();
    _controller = _makeController();
  }

  /// Calls the factory, converting a throw (registry unavailable) into a
  /// rendered error panel plus a harmless throwaway controller. Without
  /// this, initState rethrows and the whole tab crashes.
  ChainSessionController _makeController() {
    try {
      _factoryError = null;
      return widget.controllerFactory();
    } catch (e) {
      _factoryError =
          'The probe chain could not start: $e\n\n'
          'The interview bench tab keeps working. For the fix, see the '
          'console output: the sqlite3.dll placement is printed at startup.';
      return ChainSessionController(engine: DefaultProbeChainEngine());
    }
  }

  @override
  void dispose() {
    _topicController.dispose();
    _answerController.dispose();
    _skipReasonController.dispose();
    super.dispose();
  }

  ProbeChainState? get _state => _controller.state;

  Future<void> _guarded(Future<void> Function() action) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await action();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _begin() async {
    final topic = _topicController.text.trim();
    if (topic.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('The chain needs a topic.')),
      );
      return;
    }
    await _guarded(() async {
      _controller = _makeController();
      if (_factoryError != null) return;
      _landed = false;
      await _controller.begin(mode: _mode, topic: topic);
    });
  }

  Future<void> _sendAnswer() async {
    final text = _answerController.text;
    if (text.trim().isEmpty || _state == null) return;
    _answerController.clear();
    await _guarded(() => _controller.submitAnswer(text));
  }

  Future<void> _skip() async {
    final reason = _skipReasonController.text.trim();
    if (reason.isEmpty || _state == null) return;
    Navigator.of(context).pop(); // close the skip dialog
    await _guarded(() => _controller.skipCurrent(reason));
  }

  void _endAndFile() {
    if (_state == null || _landed) return;
    setState(() {
      _controller.land(sessionClosed: true);
      _landed = true;
    });
  }

  void _newChain() {
    setState(() {
      _controller = _makeController();
      _landed = false;
      _answerController.clear();
    });
  }

  void _openSkipDialog() {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Skip this level'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              '"I don\'t remember" is a legal answer. It is recorded with '
              'the level and the reason, and at sensory/source/contrast it '
              'poisons self-closure — visibly, forever.',
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _skipReasonController,
              maxLines: 2,
              decoration: const InputDecoration(
                labelText: 'Reason (kept verbatim)',
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: _skip,
            child: const Text('Record skip'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (widget.registryError != null || _factoryError != null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Probe chain')),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.storage, size: 40),
                const SizedBox(height: 12),
                const Text('The intake registry is unavailable.'),
                const SizedBox(height: 8),
                SelectableText(
                  widget.registryError ?? _factoryError ?? '',
                ),
              ],
            ),
          ),
        ),
      );
    }

    final state = _state;
    return Scaffold(
      appBar: AppBar(
        title: Text(state == null
            ? 'Probe chain - the Leeds walk'
            : 'Probe chain - ${state.topic}'),
      ),
      body: Column(
        children: [
          if (state == null)
            Expanded(child: _StartForm(screen: this))
          else ...[
            _LevelPips(state: state),
            if (_busy) const LinearProgressIndicator(minHeight: 3),
            Expanded(
              child: _landed
                  ? _HarvestPanel(controller: _controller, onNew: _newChain)
                  : _ChainView(controller: _controller, screen: this),
            ),
            if (!_landed)
              _AnswerBar(
                screen: this,
                enabled: !_busy,
                isComplete: state.isComplete,
                onEndAndFile: _endAndFile,
              ),
          ],
        ],
      ),
    );
  }
}

// --- start form -----------------------------------------------------------

class _StartForm extends StatelessWidget {
  const _StartForm({required this.screen});

  final _ProbeChainScreenState screen;

  static const Map<ElicitationMode, String> modeLabels = {
    ElicitationMode.photo: 'Photo',
    ElicitationMode.document: 'Document',
    ElicitationMode.audio: 'Audio',
    ElicitationMode.freeRecall: 'Free recall',
    ElicitationMode.chapter: 'Chapter',
  };

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: Card(
          margin: const EdgeInsets.all(24),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text('Begin the walk',
                    style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                const Text(
                  'Five levels, one at a time: surface, sensory, source, '
                  'contrast, meaning. The chain can be attempted, never '
                  'forced — and skipping is visible forever.',
                ),
                const SizedBox(height: 20),
                DropdownButtonFormField<ElicitationMode>(
                  value: screen._mode,
                  decoration: const InputDecoration(
                    labelText: 'What kind of material is this?',
                    border: OutlineInputBorder(),
                  ),
                  items: [
                    for (final mode in ElicitationMode.values)
                      DropdownMenuItem(
                        value: mode,
                        child: Text(modeLabels[mode]!),
                      ),
                  ],
                  onChanged: screen._busy
                      ? null
                      : screen._setMode,
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: screen._topicController,
                  enabled: !screen._busy,
                  decoration: const InputDecoration(
                    labelText: 'Topic (e.g. "the summer at Leeds Point")',
                    border: OutlineInputBorder(),
                  ),
                  onSubmitted: (_) => screen._begin(),
                ),
                const SizedBox(height: 20),
                FilledButton.icon(
                  onPressed: screen._busy ? null : screen._begin,
                  icon: const Icon(Icons.route),
                  label: Text(screen._busy
                      ? 'Calli is finding her words...'
                      : 'Begin chain'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// --- level pips ------------------------------------------------------------

class _LevelPips extends StatelessWidget {
  const _LevelPips({required this.state});

  final ProbeChainState state;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.fromLTRB(12, 12, 12, 0),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                for (final level in ProbeLevel.values)
                  _Pip(
                    level: level,
                    status: _statusFor(level),
                  ),
              ],
            ),
            const SizedBox(height: 6),
            Center(
              child: Text(
                'L${state.currentLevel.index + 1} · '
                '${state.currentLevel.wireValue}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ],
        ),
      ),
    );
  }

  _PipStatus _statusFor(ProbeLevel level) {
    if (state.hasSubstantiveTurn(level)) return _PipStatus.filled;
    if (state.hasSkipAt(level)) return _PipStatus.struck;
    if (level == state.currentLevel) return _PipStatus.current;
    return _PipStatus.future;
  }
}

enum _PipStatus { filled, struck, current, future }

class _Pip extends StatelessWidget {
  const _Pip({required this.level, required this.status});

  final ProbeLevel level;
  final _PipStatus status;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    const size = 22.0;

    Widget dot;
    switch (status) {
      case _PipStatus.filled:
        dot = Container(
          width: size,
          height: size,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: scheme.primary,
          ),
        );
        break;
      case _PipStatus.struck:
        dot = Stack(
          alignment: Alignment.center,
          children: [
            Container(
              width: size,
              height: size,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                border: Border.all(color: scheme.error, width: 2),
              ),
            ),
            // The strike: a rotated bar across the circle. Skips are never
            // hidden — this pip reads as a scar, not a checkbox.
            Container(width: size + 6, height: 2, color: scheme.error),
          ],
        );
        break;
      case _PipStatus.current:
        dot = Container(
          width: size,
          height: size,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(color: scheme.primary, width: 2),
          ),
          child: Padding(
            padding: const EdgeInsets.all(5),
            child: Container(
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: scheme.primaryContainer,
              ),
            ),
          ),
        );
        break;
      case _PipStatus.future:
        dot = Container(
          width: size,
          height: size,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(
                color: scheme.outlineVariant, width: 1.5),
          ),
        );
        break;
    }

    return Tooltip(
      message: 'L${level.index + 1} ${level.wireValue}',
      child: Column(
        children: [
          dot,
          const SizedBox(height: 4),
          Text(
            level.wireValue,
            style: Theme.of(context).textTheme.labelSmall,
          ),
        ],
      ),
    );
  }
}

// --- chain view ------------------------------------------------------------

class _ChainView extends StatelessWidget {
  const _ChainView({required this.controller, required this.screen});

  final ChainSessionController controller;
  final _ProbeChainScreenState screen;

  @override
  Widget build(BuildContext context) {
    final state = controller.state!;
    final closure = controller.closureStatus()!;

    return ListView(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      children: [
        _QuestionCard(
          level: state.currentLevel,
          question: state.pendingQuestion,
          source: state.pendingQuestionSource,
          discardCount: state.personaDiscards.length,
        ),
        const SizedBox(height: 8),
        for (final turn in state.turns) _TurnTile(turn: turn),
        for (final skip in state.skips) _SkipTile(skip: skip),
        const SizedBox(height: 8),
        Card(
          child: ListTile(
            dense: true,
            leading: Icon(
              closure.selfCloseEligible
                  ? Icons.verified_outlined
                  : Icons.lock_outline,
              color: closure.selfCloseEligible ? Colors.green.shade700 : null,
            ),
            title: Text(closure.reason, style: const TextStyle(fontSize: 13)),
          ),
        ),
        const SizedBox(height: 64),
      ],
    );
  }
}

class _QuestionCard extends StatelessWidget {
  const _QuestionCard({
    required this.level,
    required this.question,
    required this.source,
    required this.discardCount,
  });

  final ProbeLevel level;
  final String question;
  final QuestionSource source;
  final int discardCount;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Card(
      color: scheme.primaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: [
                _Chip(label: 'L${level.index + 1} · ${level.wireValue}'),
                _Chip(label: _sourceLabel(source)),
                if (discardCount > 0)
                  _Chip(label: '$discardCount discard(s) audited'),
              ],
            ),
            const SizedBox(height: 10),
            SelectableText(
              question,
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ],
        ),
      ),
    );
  }

  static String _sourceLabel(QuestionSource source) {
    switch (source) {
      case QuestionSource.modelProposal:
        return 'wording: Calli (model)';
      case QuestionSource.retryProposal:
        return 'wording: model, after a discard';
      case QuestionSource.staticBank:
        return 'wording: static bank';
      case QuestionSource.reask:
        return 're-ask';
    }
  }
}

class _Chip extends StatelessWidget {
  const _Chip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: scheme.surface,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: scheme.outlineVariant),
      ),
      child: Text(
        label,
        style: Theme.of(context).textTheme.labelSmall,
      ),
    );
  }
}

class _TurnTile extends StatelessWidget {
  const _TurnTile({required this.turn});

  final ProbeTurn turn;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text('L${turn.level.index + 1} · ${turn.level.wireValue}',
                    style: Theme.of(context).textTheme.labelSmall),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    turn.question,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        fontStyle: FontStyle.italic),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            SelectableText(turn.answerVerbatim),
          ],
        ),
      ),
    );
  }
}

class _SkipTile extends StatelessWidget {
  const _SkipTile({required this.skip});

  final SkipEvent skip;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Card(
      color: scheme.errorContainer.withOpacity(0.35),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'L${skip.level.index + 1} · ${skip.level.wireValue} — skipped',
              style: Theme.of(context)
                  .textTheme
                  .labelSmall
                  ?.copyWith(color: scheme.error),
            ),
            const SizedBox(height: 4),
            Text(skip.reason,
                style: const TextStyle(fontStyle: FontStyle.italic)),
          ],
        ),
      ),
    );
  }
}

// --- answer bar -------------------------------------------------------------

class _AnswerBar extends StatelessWidget {
  const _AnswerBar({
    required this.screen,
    required this.enabled,
    required this.isComplete,
    required this.onEndAndFile,
  });

  final _ProbeChainScreenState screen;
  final bool enabled;
  final bool isComplete;
  final VoidCallback onEndAndFile;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 4, 12, 12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                Expanded(
                  child: TextField(
                    controller: screen._answerController,
                    enabled: enabled,
                    minLines: 1,
                    maxLines: 4,
                    decoration: const InputDecoration(
                      hintText:
                          'Your answer (voice-to-text works in this field)...',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    onSubmitted: (_) => screen._sendAnswer(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.filledTonal(
                  onPressed: enabled ? screen._openSkipDialog : null,
                  tooltip: 'Skip - "I don\'t remember" is a legal answer',
                  icon: const Icon(Icons.skip_next),
                ),
                const SizedBox(width: 4),
                FilledButton(
                  onPressed: enabled ? screen._sendAnswer : null,
                  child: const Text('Send'),
                ),
              ],
            ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: enabled ? onEndAndFile : null,
                icon: const Icon(Icons.inventory_2_outlined),
                label: Text(isComplete
                    ? 'File the harvest (end of chain)'
                    : 'End early & file what exists'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// --- harvest panel ----------------------------------------------------------

class _HarvestPanel extends StatelessWidget {
  const _HarvestPanel({required this.controller, required this.onNew});

  final ChainSessionController controller;
  final VoidCallback onNew;

  static const Map<RecordType, String> typeLabels = {
    RecordType.event: 'event',
    RecordType.judgment: 'judgment',
    RecordType.claim: 'claim',
    RecordType.memoryCandidate: 'memory candidate',
  };

  @override
  Widget build(BuildContext context) {
    final minted = controller.lastMinted;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        Card(
          color: Theme.of(context).colorScheme.secondaryContainer,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Harvest',
                    style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 6),
                Text(
                  minted.isEmpty
                      ? 'Nothing minted: the surface level never got a '
                          'substantive answer. The session row still exists '
                          'in the registry.'
                      : '${minted.length} candidate(s) filed into the '
                          'registry. They are candidates — nothing is '
                          'approved. Approval, rejection, reopening and '
                          'closure are Dave\'s alone at the gate '
                          '(Stage 5-b-4 UI).',
                ),
              ],
            ),
          ),
        ),
        for (final candidate in minted)
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      _Chip(label: typeLabels[candidate.recordType]!),
                      _Chip(label: 'confidence '
                          '${candidate.confidence.toStringAsFixed(1)}'),
                      _Chip(label: candidate.recordId),
                    ],
                  ),
                  const SizedBox(height: 10),
                  SelectableText(candidate.canonicalText),
                  const SizedBox(height: 8),
                  Text(
                    'chain: ${candidate.probeChain.join(' -> ')}',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ),
        const SizedBox(height: 8),
        FilledButton.icon(
          onPressed: onNew,
          icon: const Icon(Icons.restart_alt),
          label: const Text('New chain'),
        ),
        const SizedBox(height: 24),
      ],
    );
  }
}
