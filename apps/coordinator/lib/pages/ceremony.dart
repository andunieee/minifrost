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
  bool confirming = false;

  /// The signer the toast on screen is about, so rescanning its reply doesn't toast again.
  String? toastFrom;

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
      final stepDone = u.finished != null || u.broadcast != null || u.awaiting.isEmpty;
      final from = u.from;
      final color = from == null ? null : signerColor(from.deviceId, Theme.of(context).brightness);
      if (u.repeat) {
        // the camera keeps reading the same reply while it's in view: say so once, then stay quiet
        if (toastFrom != from?.deviceId) showInfoToast(context, u.message, color: color);
      } else {
        showSuccessToast(context, stepDone ? '${u.message} All replies received: step complete.' : u.message,
            color: color);
      }
      toastFrom = from?.deviceId;
      // a repeat carries nothing new (no check code, no broadcast): keep what is on screen
      if (u.repeat) return;
      setState(() {
        update = u;
        message = u.message;
        if (u.broadcast != null) broadcast = u.broadcast;
      });
    } catch (e) {
      if (!mounted) return;
      toastFrom = null;
      ScaffoldMessenger.of(context)
        ..hideCurrentSnackBar()
        ..showSnackBar(SnackBar(content: Text('$e')));
    }
  }

  /// A step with no signer reply (a displayed or saved backup): the user confirms it here.
  Future<void> _confirm() async {
    setState(() => confirming = true);
    try {
      final u = await widget.app.confirmBackup();
      if (!mounted) return;
      setState(() {
        update = u;
        message = u.message;
        if (u.broadcast != null) broadcast = u.broadcast;
      });
      if (u.finished != null) showSuccessToast(context, u.message);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context)
        ..hideCurrentSnackBar()
        ..showSnackBar(SnackBar(content: Text('$e')));
    } finally {
      if (mounted) setState(() => confirming = false);
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

  Widget _confirmation(ThemeData theme, String label) {
    return Card(
      color: theme.colorScheme.secondaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          Text(
            'When the signer has finished, confirm it here. '
            'Only confirm if you really did it: this is the coordinator’s only record.',
          ),
          const SizedBox(height: 16),
          FilledButton.icon(
            onPressed: confirming ? null : _confirm,
            icon: confirming
                ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                : const Icon(Icons.check),
            label: Text(label),
          ),
        ]),
      ),
    );
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
        body: LayoutBuilder(builder: (context, box) {
          final narrow = box.maxWidth < 900;
          final qrSize = narrow ? (box.maxWidth - 80).clamp(200.0, 380.0) : 380.0;
          final shown = <Widget>[
              // a finished signing or backup session has nothing left to show to the signers;
              // key creation still shows its last code so every signer can store its share
              if (broadcast != null && (f == null || f is Finished_KeyCreated)) ...[
                Text(broadcast!.title, style: theme.textTheme.titleLarge),
                const SizedBox(height: 4),
                Text(message),
                const SizedBox(height: 16),
                Center(
                  child: AnimatedQr(
                    frames: broadcast!.frames,
                    size: qrSize,
                    label: 'Showing: ${broadcast!.title}',
                    sound: const RustSoundPlayer(),
                  ),
                ),
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
          ];
          final scan = <Widget>[
            Text('Signers', style: theme.textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(update.awaiting.isEmpty ? 'All replies received.' : 'Waiting for ${update.awaiting.length}:'),
            const SizedBox(height: 4),
            Wrap(spacing: 8, runSpacing: 4, children: [
              for (final s in update.awaiting) SignerChip(deviceId: s.deviceId, name: s.name),
              for (final s in update.received) SignerChip(deviceId: s.deviceId, name: s.name, done: true),
            ]),
            const SizedBox(height: 16),
            ScanPanel(
              backend: const RustQrBackend(),
              onPayload: _onPayload,
              expecting: broadcast == null ? 'signer reply' : 'signer reply to “${broadcast!.title}”',
            ),
          ];
          final confirm = update.confirm;
          // a phone has no room for two columns: show the code first, the reply area below it
          if (narrow) {
            return ListView(padding: const EdgeInsets.all(16), children: [
              ...shown,
              if (!finished) ...[
                const Divider(height: 32),
                if (confirm != null) _confirmation(theme, confirm) else ...scan,
              ],
            ]);
          }
          return Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Expanded(child: ListView(padding: const EdgeInsets.all(24), children: shown)),
            const VerticalDivider(width: 1),
            SizedBox(
              width: 460,
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: finished
                    ? const SizedBox()
                    : confirm != null
                    ? _confirmation(theme, confirm)
                    : ListView(children: scan),
              ),
            ),
          ]);
        }),
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
          'You confirmed writing down the 25 words for “$keyName”. Keep the paper safe and secret.',
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
