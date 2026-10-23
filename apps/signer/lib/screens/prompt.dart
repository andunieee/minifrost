import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../src/rust/engine.dart';

/// Asks the user to approve or reject. [onApprove] / [onReject] do the work and close the screen.
class PromptScreen extends StatefulWidget {
  const PromptScreen({super.key, required this.prompt, required this.onApprove, required this.onReject});
  final Prompt prompt;
  final Future<void> Function() onApprove;
  final Future<void> Function() onReject;

  @override
  State<PromptScreen> createState() => _PromptScreenState();
}

class _PromptScreenState extends State<PromptScreen> {
  bool busy = false;

  Future<void> _run(Future<void> Function() f) async {
    setState(() => busy = true);
    try {
      await f();
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = widget.prompt;
    final (title, body, approve) = switch (p) {
      Prompt_JoinKeygen(:final keyName, :final threshold, :final nSigners, :final network) => (
          'New key',
          _text('Create the ${network.toUpperCase()} wallet “$keyName”.\n\n'
              '$threshold of $nSigners signers will be needed to spend from it. '
              'Approve only if you started this on your coordinator.'),
          'Join'
        ),
      Prompt_VerifyKeygen(:final keyName, :final threshold, :final nSigners, :final checkCode) => (
          'Compare check code',
          _checkCode(context, keyName, threshold, nSigners, checkCode),
          'Codes match'
        ),
      Prompt_ReviewTransaction(:final field0) => (
          'Review transaction',
          _tx(field0, 'Approving releases fresh nonces so signing can start. Nothing is signed yet.'),
          'Release nonces'
        ),
      Prompt_SignTransaction(:final field0) => (
          'Sign transaction',
          _tx(field0, 'Approving produces your partial signature. This cannot be undone.'),
          'Sign'
        ),
      Prompt_DisplayBackup(:final keyName, :final threshold, :final shareIndex, :final words) => (
          'Secret backup',
          _backup(context, keyName, threshold, shareIndex, words),
          'I wrote it down'
        ),
    };
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: ListView(padding: const EdgeInsets.all(16), children: [
        body,
        const SizedBox(height: 24),
        Row(children: [
          Expanded(
            child: OutlinedButton(
              onPressed: busy ? null : () => _run(widget.onReject),
              child: const Text('Reject'),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: FilledButton(
              onPressed: busy ? null : () => _run(widget.onApprove),
              child: busy ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2)) : Text(approve),
            ),
          ),
        ]),
      ]),
    );
  }

  Widget _backup(BuildContext c, String name, int threshold, int shareIndex, List<String> words) {
    final scheme = Theme.of(c).colorScheme;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(color: scheme.errorContainer, borderRadius: BorderRadius.circular(8)),
        child: Row(children: [
          Icon(Icons.warning_amber_rounded, color: scheme.onErrorContainer),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              'This backup is secret. Anyone holding $threshold of the wallet’s backups can take all of its bitcoin. '
              'Write the words down on paper, in order. Do not photograph or type them anywhere.',
              style: TextStyle(color: scheme.onErrorContainer, fontWeight: FontWeight.bold),
            ),
          ),
        ]),
      ),
      const SizedBox(height: 16),
      Text('Wallet “$name” · $threshold signatures needed', style: Theme.of(c).textTheme.bodyLarge),
      const SizedBox(height: 4),
      Text('Key #$shareIndex', style: Theme.of(c).textTheme.headlineMedium),
      const SizedBox(height: 12),
      Wrap(spacing: 8, runSpacing: 8, children: [
        for (var i = 0; i < words.length; i++)
          SizedBox(
            width: 150,
            child: Row(children: [
              SizedBox(width: 28, child: Text('${i + 1}', textAlign: TextAlign.right, style: TextStyle(color: scheme.outline))),
              const SizedBox(width: 8),
              Text(words[i], style: const TextStyle(fontFamily: 'monospace', fontSize: 18, fontWeight: FontWeight.bold)),
            ]),
          ),
      ]),
      const SizedBox(height: 12),
      const Text('Also note the key number, the threshold and the wallet name next to the words.'),
    ]);
  }

  Widget _text(String s) => Padding(padding: const EdgeInsets.all(8), child: Text(s, style: const TextStyle(fontSize: 16)));

  Widget _checkCode(BuildContext c, String name, int t, int n, String code) => Column(children: [
        _text('Key “$name” ($t of $n).\n\nEvery signer and the coordinator must show exactly this code. '
            'If any differs, reject: someone is not seeing the same key.'),
        const SizedBox(height: 16),
        Text(code, style: const TextStyle(fontFamily: 'monospace', fontSize: 34, fontWeight: FontWeight.bold)),
      ]);

  Widget _tx(TxSummary s, String note) => Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text('Wallet “${s.keyName}” · ${s.network}', style: const TextStyle(fontWeight: FontWeight.bold)),
        const SizedBox(height: 8),
        TxCard(
          outputs: [for (final o in s.outputs) TxOutputRow(address: o.address, sats: o.amountSats.toInt(), ownLabel: o.ownLabel)],
          feeSats: s.feeSats.toInt(),
          feeRate: s.feeRate,
          nInputs: s.nInputs,
          txid: s.txid,
          warnings: s.warnings,
          verified: s.verified,
        ),
        const SizedBox(height: 8),
        Text(note),
      ]);
}
