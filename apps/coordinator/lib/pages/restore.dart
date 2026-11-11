import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';
import '../src/rust/api.dart';
import '../src/rust/engine.dart';
import '../src/rust/engine/restore.dart';
import 'ceremony.dart';

/// Rebuild a wallet from at least a threshold of its shares. Restoring starts on the signers: each
/// shows a restore code, for a backup typed into it or a key it holds, and this page reads them
/// one after another. The coordinator sorts them out and saves the wallet once it has enough.
class RestorePage extends StatefulWidget {
  const RestorePage({super.key, required this.app});
  final CoordinatorApp app;
  @override
  State<RestorePage> createState() => _RestorePageState();
}

class _RestorePageState extends State<RestorePage> {
  late Future<_RestoreData> data = _load();
  bool scanning = false;

  /// The signer the toast on screen is about, so rereading its code doesn't toast again.
  String? toastFrom;

  Future<_RestoreData> _load() async => _RestoreData(
        await widget.app.restoringShares(),
        await widget.app.unnamedWallets(),
        await widget.app.pendingConsolidations(),
      );

  void _reload() => setState(() {
    data = _load();
  });

  /// Run [action] and report its error as a snackbar; reloads afterwards either way.
  Future<void> _guard(Future<void> Function() action) async {
    try {
      await action();
    } catch (e) {
      if (mounted) toast(context, e);
    }
    if (mounted) _reload();
  }

  Future<void> _onPayload(Uint8List bytes) => _guard(() async {
        final u = await widget.app.readRestoreCode(payload: bytes);
        if (!mounted) return;
        final from = u.from;
        final color = from == null ? null : signerColor(from.deviceId, Theme.of(context).brightness);
        // the camera keeps reading the same code while it's in view: say so once, then stay quiet
        if (!u.repeat) {
          showSuccessToast(context, u.message, color: color);
        } else if (toastFrom != from?.deviceId) {
          showInfoToast(context, u.message, color: color);
        }
        toastFrom = from?.deviceId;
        await _next(u);
      });

  /// What a code or a step here led to: naming the wallet it rebuilt, then the code for the signers.
  Future<void> _next(Update u) async {
    final unnamed = u.unnamedWallet;
    if (unnamed != null) {
      await _name(unnamed);
    } else if (u.broadcast != null) {
      await _ceremony(u);
    } else if (u.finished != null && scanning) {
      setState(() => scanning = false);
    }
  }

  Future<void> _ceremony(Update u) async {
    // the page behind doesn't need the camera, and the wallet this was for is done
    setState(() => scanning = false);
    await Navigator.push<Finished?>(
      context,
      MaterialPageRoute(builder: (_) => CeremonyPage(app: widget.app, title: 'Finish restoring', initial: u)),
    );
  }

  Future<void> _name(UnnamedWalletView w) async {
    final details = await showDialog<_WalletDetails>(
      context: context,
      builder: (_) => _WalletDetailsDialog(
        title: 'Name the rebuilt wallet',
        intro: '${w.threshold} signatures needed, read from ${w.signers.join(', ')}. '
            'A backup doesn’t say what the wallet was called or which network it is on: use your backup sheet.',
      ),
    );
    if (details == null || !mounted) return;
    final u = await widget.app.nameRestoredWallet(keyId: w.keyId, name: details.name, network: details.network);
    if (!mounted) return;
    showSuccessToast(context, u.message);
    await _next(u);
  }

  Future<void> _oneSignature(RestoringShareView s) => _guard(() async {
        final details = await showDialog<_WalletDetails>(
          context: context,
          builder: (_) => const _WalletDetailsDialog(
            title: 'Restore a one-signature wallet',
            intro: 'Do this only if your backup sheet says one signature is enough. Nothing can check that '
                'from a single backup, and a wallet of a higher threshold rebuilds by itself once you read enough codes.',
          ),
        );
        if (details == null || !mounted) return;
        final u = await widget.app.restoreOneSignatureWallet(id: s.id, name: details.name, network: details.network);
        if (!mounted) return;
        showSuccessToast(context, u.message);
        await _next(u);
      });

  Future<void> _showCode(String keyId) => _guard(() async {
        final u = await widget.app.startFinishRestoring(keyId: keyId);
        if (mounted) await _ceremony(u);
      });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('Restore')),
      body: FutureBuilder(
        future: data,
        builder: (context, snap) {
          final d = snap.data;
          if (d == null) return const Center(child: CircularProgressIndicator());
          // the same wallet can have several signers waiting; one code serves them all
          final waiting = <String, List<PendingConsolidationView>>{};
          for (final p in d.pending) {
            (waiting[p.keyId] ??= []).add(p);
          }
          return ListView(padding: const EdgeInsets.all(16), children: [
            Text(
              'Rebuild a wallet from at least a threshold of its shares. On each signer, type in a backup '
              '(25 words) or tap a key and choose “Restore wallet on coordinator”, then read the code it shows here. '
              'The wallet is saved as soon as there are enough.',
              style: theme.textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.icon(
                onPressed: () => setState(() => scanning = !scanning),
                icon: Icon(scanning ? Icons.close : Icons.qr_code_scanner),
                label: Text(scanning ? 'Stop reading' : 'Read a signer’s code'),
              ),
            ),
            if (scanning) ...[
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerLeft,
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 480),
                  child: ScanPanel(
                    backend: const RustQrBackend(),
                    onPayload: _onPayload,
                    hint: 'Show the signer’s restore code to the camera',
                    expecting: 'signer restore code',
                  ),
                ),
              ),
            ],
            for (final w in d.unnamed)
              Card(
                margin: const EdgeInsets.only(top: 16),
                child: ListTile(
                  leading: Icon(Icons.check_circle, color: Colors.green.shade700),
                  title: const Text('Enough codes to rebuild a wallet'),
                  subtitle: Text('${w.threshold} signatures needed, read from ${w.signers.join(', ')}'),
                  trailing: FilledButton(onPressed: () => _guard(() => _name(w)), child: const Text('Name it')),
                ),
              ),
            if (d.shares.isNotEmpty) ...[
              const SizedBox(height: 24),
              Text('Read so far', style: theme.textTheme.titleMedium),
              for (final s in d.shares)
                _ShareTile(
                  share: s,
                  onRemove: () => _guard(() => widget.app.removeRestoringShare(id: s.id)),
                  onOneSignature: () => _oneSignature(s),
                ),
            ],
            if (waiting.isNotEmpty) ...[
              const SizedBox(height: 24),
              Text('Signers waiting for their key', style: theme.textTheme.titleMedium),
              const SizedBox(height: 4),
              const Text('These signers took a backup. Until they read the wallet’s code they cannot sign.'),
              for (final ps in waiting.values)
                Card(
                  child: ListTile(
                    leading: const Icon(Icons.hourglass_bottom),
                    title: Text('Wallet “${ps.first.keyName}”'),
                    subtitle: Text(ps.map((p) => '${p.deviceName} · key #${p.shareIndex}').join('\n')),
                    trailing: FilledButton(onPressed: () => _showCode(ps.first.keyId), child: const Text('Show code')),
                  ),
                ),
            ],
          ]);
        },
      ),
    );
  }
}

class _RestoreData {
  _RestoreData(this.shares, this.unnamed, this.pending);
  final List<RestoringShareView> shares;
  final List<UnnamedWalletView> unnamed;
  final List<PendingConsolidationView> pending;
}

class _ShareTile extends StatelessWidget {
  const _ShareTile({required this.share, required this.onRemove, required this.onOneSignature});
  final RestoringShareView share;
  final VoidCallback onRemove;
  final VoidCallback onOneSignature;

  @override
  Widget build(BuildContext context) {
    final color = signerColor(share.deviceId, Theme.of(context).brightness);
    final name = share.keyName;
    final network = share.network;
    final threshold = share.threshold;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(Icons.vpn_key_outlined, color: color),
      title: Text('Key #${share.shareIndex} on ${share.deviceName}'),
      subtitle: Text(name == null
          ? 'From a backup: its wallet shows once more codes are read.'
          : [
              '“$name”',
              if (network != null) networkLabel(network),
              if (threshold != null) '$threshold signatures needed',
            ].join(' · ')),
      trailing: Row(mainAxisSize: MainAxisSize.min, children: [
        if (threshold == null) TextButton(onPressed: onOneSignature, child: const Text('Only one signature?')),
        IconButton(icon: const Icon(Icons.close), tooltip: 'Remove this share', onPressed: onRemove),
      ]),
    );
  }
}

class _WalletDetails {
  _WalletDetails(this.name, this.network);
  final String name;
  final String network;
}

class _WalletDetailsDialog extends StatefulWidget {
  const _WalletDetailsDialog({required this.title, required this.intro});
  final String title;
  final String intro;
  @override
  State<_WalletDetailsDialog> createState() => _WalletDetailsDialogState();
}

class _WalletDetailsDialogState extends State<_WalletDetailsDialog> {
  final name = TextEditingController();
  String network = 'signet';

  @override
  void dispose() {
    name.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(widget.intro),
            const SizedBox(height: 12),
            TextField(
              controller: name,
              autofocus: true,
              decoration: const InputDecoration(labelText: 'Wallet name'),
              onChanged: (_) => setState(() {}),
            ),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              initialValue: network,
              decoration: const InputDecoration(labelText: 'Network'),
              items: const [
                DropdownMenuItem(value: 'signet', child: Text('Mutinynet')),
                DropdownMenuItem(value: 'testnet', child: Text('Testnet')),
                DropdownMenuItem(value: 'bitcoin', child: Text('Bitcoin mainnet')),
              ],
              onChanged: (v) => setState(() => network = v ?? network),
            ),
          ]),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Not now')),
        FilledButton(
          onPressed: name.text.trim().isEmpty ? null : () => Navigator.pop(context, _WalletDetails(name.text.trim(), network)),
          child: const Text('Restore wallet'),
        ),
      ],
    );
  }
}
