import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';
import '../src/rust/api.dart';
import '../src/rust/engine.dart';

/// One keygen or signing ceremony: show the current broadcast QR to the signers, scan their
/// replies, and move on when the coordinator has everything it needs for the next round.
class CeremonyPage extends StatefulWidget {
  const CeremonyPage({super.key, required this.app, required this.title, required this.initial, this.keyId});
  final CoordinatorApp app;
  final String title;
  final Update initial;

  /// Needed for broadcasting a signed transaction.
  final String? keyId;

  @override
  State<CeremonyPage> createState() => _CeremonyPageState();
}

class _CeremonyPageState extends State<CeremonyPage> {
  late Update update = widget.initial;
  Broadcast? broadcast;
  String message = '';
  String? broadcastResult;
  String? broadcastError;
  bool broadcasting = false;

  @override
  void initState() {
    super.initState();
    broadcast = update.broadcast;
    message = update.message;
  }

  bool get finished => update.finished != null;

  Future<void> _onPayload(Uint8List bytes) async {
    try {
      final u = await widget.app.receive(payload: bytes);
      if (!mounted) return;
      setState(() {
        update = u;
        message = u.message;
        if (u.broadcast != null) broadcast = u.broadcast;
      });
    } catch (e) {
      if (mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
    }
  }

  Future<void> _cancel() async {
    if (!finished) {
      final ok = await showDialog<bool>(
        context: context,
        builder: (_) => AlertDialog(
          title: const Text('Cancel this session?'),
          content: const Text('Progress with the signers will be discarded.'),
          actions: [
            TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Keep going')),
            FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Cancel session')),
          ],
        ),
      );
      if (ok != true) return;
      await widget.app.cancelSession();
    }
    if (mounted) Navigator.pop(context, update.finished);
  }

  Future<void> _broadcast(String txHex) async {
    setState(() {
      broadcasting = true;
      broadcastError = null;
    });
    try {
      final txid = await widget.app.broadcast(keyId: widget.keyId!, txHex: txHex);
      if (mounted) setState(() => broadcastResult = txid);
    } catch (e) {
      if (mounted) setState(() => broadcastError = '$e');
    } finally {
      if (mounted) setState(() => broadcasting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final f = update.finished;
    final theme = Theme.of(context);
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _cancel();
      },
      child: Scaffold(
        appBar: AppBar(
          title: Text(widget.title),
          leading: IconButton(icon: const Icon(Icons.close), onPressed: _cancel),
        ),
        body: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Expanded(
            child: ListView(padding: const EdgeInsets.all(24), children: [
              if (broadcast != null) ...[
                Text(broadcast!.title, style: theme.textTheme.titleLarge),
                const SizedBox(height: 4),
                Text(message),
                const SizedBox(height: 16),
                Center(child: AnimatedQr(frames: broadcast!.frames, size: 380, sound: const RustSoundPlayer())),
              ],
              if (update.checkCode != null) ...[
                const SizedBox(height: 24),
                Card(
                  color: theme.colorScheme.primaryContainer,
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(children: [
                      const Text('Check code — must be identical on every signer'),
                      const SizedBox(height: 8),
                      SelectableText(update.checkCode!,
                          style: const TextStyle(fontFamily: 'monospace', fontSize: 36, fontWeight: FontWeight.bold)),
                    ]),
                  ),
                ),
              ],
              if (f is Finished_KeyCreated) ...[
                const SizedBox(height: 24),
                Card(
                  child: ListTile(
                    leading: const Icon(Icons.check_circle, color: Colors.green),
                    title: Text('Wallet “${f.name}” created'),
                    subtitle: const Text('Show the code above so every signer stores its share. Then back up every signer from the wallet page: until a backup is written down, losing a signer can lose the wallet.'),
                  ),
                ),
                FilledButton(onPressed: () => Navigator.pop(context, f), child: const Text('Done')),
              ],
              if (f is Finished_Signed) ..._signedSection(f),
              if (f != null && f is! Finished_KeyCreated && f is! Finished_Signed) ..._backupSection(f),
            ]),
          ),
          const VerticalDivider(width: 1),
          SizedBox(
            width: 460,
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: finished
                  ? const SizedBox()
                  : ListView(children: [
                      Text('Signers', style: theme.textTheme.titleMedium),
                      const SizedBox(height: 4),
                      Text(update.awaiting.isEmpty
                          ? 'All replies received.'
                          : 'Waiting for: ${update.awaiting.join(', ')}'),
                      const SizedBox(height: 16),
                      ScanPanel(backend: const RustQrBackend(), onPayload: _onPayload),
                    ]),
            ),
          ),
        ]),
      ),
    );
  }

  /// Result cards for the backup and restoration sessions.
  List<Widget> _backupSection(Finished f) {
    final scheme = Theme.of(context).colorScheme;
    final (IconData icon, Color color, String title, String detail) = switch (f) {
      Finished_BackupRecorded(:final keyName, :final shareIndex) => (
          Icons.check_circle,
          Colors.green,
          'Backup of key #$shareIndex recorded',
          'The user confirmed writing down the 25 words for “$keyName”. Keep the paper safe and secret.',
        ),
      Finished_BackupChecked(:final keyName, :final shareIndex) => (
          Icons.verified,
          Colors.green,
          'Backup of key #$shareIndex checked',
          'Every answer matched the signer’s share of “$keyName”.',
        ),
      Finished_BackupSaved(:final shareIndex, :final restorable) => (
          Icons.check_circle,
          Colors.green,
          'Backup #$shareIndex saved on the signer',
          restorable
              ? 'There are enough shares to rebuild the wallet. Finish the restoration from the Restore page.'
              : 'More backups are needed before the wallet can be rebuilt.',
        ),
      Finished_BackupRejected(:final reason) => (Icons.error_outline, scheme.error, 'Backup not accepted', reason),
      Finished_Consolidated(:final keyName, :final shareIndex) => (
          Icons.check_circle,
          Colors.green,
          'Key #$shareIndex is ready',
          'The signer now holds its share of “$keyName” and can sign.',
        ),
      Finished_HeldShares(:final deviceName, :final shares) => (
          shares.isEmpty ? Icons.info_outline : Icons.check_circle,
          shares.isEmpty ? scheme.outline : Colors.green,
          '$deviceName holds ${shares.length} share(s)',
          shares.isEmpty ? 'Nothing to restore from this signer.' : 'Close this page to choose one.',
        ),
      _ => (Icons.check_circle, Colors.green, 'Done', ''),
    };
    return [
      const SizedBox(height: 24),
      Card(child: ListTile(leading: Icon(icon, color: color), title: Text(title), subtitle: Text(detail))),
      const SizedBox(height: 8),
      FilledButton(onPressed: () => Navigator.pop(context, f), child: const Text('Close')),
    ];
  }

  List<Widget> _signedSection(Finished_Signed f) => [
        const SizedBox(height: 24),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              const ListTile(
                contentPadding: EdgeInsets.zero,
                leading: Icon(Icons.check_circle, color: Colors.green),
                title: Text('Signed by the threshold of signers'),
                subtitle: Text('The signature is valid. Nothing has been published yet.'),
              ),
              SelectableText('txid ${f.txid}', style: const TextStyle(fontFamily: 'monospace')),
              const SizedBox(height: 12),
              Wrap(spacing: 8, children: [
                FilledButton.icon(
                  onPressed: broadcasting || broadcastResult != null ? null : () => _broadcast(f.txHex),
                  icon: broadcasting
                      ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                      : const Icon(Icons.public),
                  label: const Text('Publish via Esplora'),
                ),
                OutlinedButton.icon(
                  onPressed: () => Clipboard.setData(ClipboardData(text: f.txHex)),
                  icon: const Icon(Icons.copy),
                  label: const Text('Copy raw transaction'),
                ),
              ]),
              if (broadcastResult != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: SelectableText('Published. txid $broadcastResult',
                      style: const TextStyle(color: Colors.green, fontFamily: 'monospace')),
                ),
              if (broadcastError != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(broadcastError!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
                ),
            ]),
          ),
        ),
        const SizedBox(height: 12),
        OutlinedButton(onPressed: () => Navigator.pop(context, f), child: const Text('Close')),
      ];
}
