import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:minifrost_ui/minifrost_ui.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../src/rust/api.dart';
import '../src/rust/engine.dart';
import '../src/rust/engine/restore.dart';
import 'ceremony.dart';
import 'pick_signer.dart';

class WalletDetailPage extends StatefulWidget {
  const WalletDetailPage({super.key, required this.app, required this.wallet});
  final CoordinatorApp app;
  final KeyView wallet;
  @override
  State<WalletDetailPage> createState() => _WalletDetailPageState();
}

class _WalletDetailPageState extends State<WalletDetailPage> {
  late KeyView wallet = widget.wallet;
  List<BackupShare> backups = [];
  List<PendingConsolidationView> pending = [];
  WalletSummary? summary;
  String? error;
  bool loading = false;

  final to = TextEditingController();
  final amount = TextEditingController();
  final feeRate = TextEditingController(text: '2');
  bool sendMax = false;
  bool busy = false;

  @override
  void initState() {
    super.initState();
    _loadBackups();
    _refresh();
  }

  Future<void> _loadBackups() async {
    try {
      final keys = await widget.app.keys();
      final b = await widget.app.backupShares(keyId: wallet.keyId);
      final p = (await widget.app.pendingConsolidations()).where((p) => p.keyId == wallet.keyId).toList();
      if (!mounted) return;
      setState(() {
        wallet = keys.firstWhere((k) => k.keyId == wallet.keyId, orElse: () => wallet);
        backups = b;
        pending = p;
      });
    } catch (e) {
      toast(context, e);
    }
  }

  /// Open a backup session for the signers, then refresh what the card shows.
  Future<void> _session(String title, Future<Update> Function() start) async {
    try {
      final update = await start();
      if (!mounted) return;
      await Navigator.push<Finished?>(
        context,
        MaterialPageRoute(builder: (_) => CeremonyPage(app: widget.app, title: title, initial: update)),
      );
    } catch (e) {
      if (mounted) toast(context, e);
    }
    if (mounted) _loadBackups();
  }

  Future<void> _markBackedUp(BackupShare s) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('Mark as backed up?'),
        content: Text(
          'Only do this if you have already recorded the backup for key #${s.shareIndex} somewhere safe.\n\n'
          'If you mark a key as backed up without actually having the backup, you risk permanently losing access to this wallet.',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('I have the backup')),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await widget.app.markBackupComplete(keyId: wallet.keyId, shareIndex: s.shareIndex);
    } catch (e) {
      if (mounted) toast(context, e);
    }
    if (mounted) _loadBackups();
  }

  /// A signer that was replaced or wiped rejoins the wallet through its paper backup.
  Future<void> _addSignerFromBackup() async {
    final all = await widget.app.signers();
    if (!mounted) return;
    final candidates = all.where((s) => !wallet.signers.any((w) => w.deviceId == s.deviceId)).toList();
    final signer = await pickSigner(
      context,
      signers: candidates,
      title: 'Which signer takes the backup?',
      hint: 'A signer that does not hold a share of this wallet yet. You will type the backup into it.',
    );
    if (signer == null || !mounted) return;
    await _session(
      'Add ${signer.name} from a backup',
      () => widget.app.startEnterBackupWallet(deviceId: signer.deviceId, keyId: wallet.keyId),
    );
  }

  Widget _backupCard(ThemeData theme) {
    final missing = backups.where((b) => !b.complete).length;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Row(children: [
            Icon(missing > 0 ? Icons.warning_amber_rounded : Icons.verified_outlined,
                color: missing > 0 ? theme.colorScheme.error : Colors.green),
            const SizedBox(width: 8),
            Expanded(child: Text('Backups', style: theme.textTheme.titleMedium)),
          ]),
          const SizedBox(height: 4),
          Text(
            missing > 0
                ? 'Write down every signer’s 25 words. Without backups, losing ${wallet.signers.length - wallet.threshold + 1} '
                    'signer(s) loses the wallet.'
                : 'Every key has a recorded backup. Check them from time to time.',
          ),
          for (final b in backups)
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(b.complete ? Icons.check_circle : Icons.radio_button_unchecked,
                  color: b.complete ? Colors.green : theme.colorScheme.error),
              title: Text('${b.name} · key #${b.shareIndex}'),
              subtitle: Text(b.complete ? 'Backed up' : 'Not backed up'),
              trailing: Wrap(spacing: 4, children: [
                if (!b.complete)
                  TextButton(onPressed: () => _markBackedUp(b), child: const Text('Mark as backed up')),
                if (b.complete)
                  OutlinedButton(
                    onPressed: () => _session(
                      'Check ${b.name}’s backup',
                      () => widget.app.startCheckBackup(keyId: wallet.keyId, deviceId: b.deviceId),
                    ),
                    child: const Text('Check'),
                  ),
                FilledButton.tonal(
                  onPressed: () => _session(
                    'Back up ${b.name}',
                    () => widget.app.startDisplayBackup(keyId: wallet.keyId, deviceId: b.deviceId),
                  ),
                  child: Text(b.complete ? 'Show again' : 'Show backup'),
                ),
              ]),
            ),
          for (final p in pending)
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.hourglass_bottom),
              title: Text('${p.deviceName} · key #${p.shareIndex} is waiting to be finished'),
              subtitle: const Text('Its typed-in backup becomes a normal share once it is finished.'),
              trailing: FilledButton(
                onPressed: () => _session(
                  'Finish ${p.deviceName}',
                  () => widget.app.startConsolidate(keyId: p.keyId, deviceId: p.deviceId),
                ),
                child: const Text('Finish'),
              ),
            ),
          const SizedBox(height: 4),
          TextButton.icon(
            onPressed: _addSignerFromBackup,
            icon: const Icon(Icons.edit_note),
            label: const Text('Add a signer from a backup'),
          ),
        ]),
      ),
    );
  }

  Future<void> _refresh() async {
    setState(() {
      loading = true;
      error = null;
    });
    try {
      final s = await widget.app.refreshWallet(keyId: wallet.keyId);
      if (mounted) setState(() => summary = s);
    } catch (e) {
      if (mounted) setState(() => error = '$e');
    } finally {
      if (mounted) setState(() => loading = false);
    }
  }

  Future<void> _suggestFee() async {
    try {
      final r = await widget.app.suggestedFeeRate(keyId: wallet.keyId, blocks: 6);
      setState(() => feeRate.text = r.toStringAsFixed(1));
    } catch (e) {
      _toast('$e');
    }
  }

  void _toast(String m) => ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(m)));

  Future<void> _send() async {
    final rate = double.tryParse(feeRate.text.trim());
    final sats = int.tryParse(amount.text.trim());
    if (rate == null) return _toast('Enter a fee rate.');
    if (!sendMax && sats == null) return _toast('Enter the amount in sats.');
    setState(() => busy = true);
    try {
      final preview = await widget.app.prepareSend(
        keyId: wallet.keyId,
        toAddress: to.text.trim(),
        amountSats: sendMax ? null : sats,
        feeRate: rate,
      );
      if (!mounted) return;
      final chosen = await showDialog<List<String>>(
        context: context,
        builder: (_) => _ConfirmSendDialog(wallet: wallet, preview: preview),
      );
      if (chosen == null || !mounted) return;
      final update = await widget.app.beginSigning(keyId: wallet.keyId, deviceIds: chosen);
      if (!mounted) return;
      await Navigator.push(
        context,
        MaterialPageRoute(
          builder: (_) => CeremonyPage(
            app: widget.app,
            title: 'Sign payment',
            initial: update,
            keyId: wallet.keyId,
          ),
        ),
      );
      _refresh();
    } catch (e) {
      _toast('$e');
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final w = wallet;
    final s = summary;
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(w.name),
        actions: [IconButton(onPressed: loading ? null : _refresh, icon: const Icon(Icons.refresh), tooltip: 'Refresh from Esplora')],
      ),
      body: ListView(padding: const EdgeInsets.all(24), children: [
        Text('${w.threshold}-of-${w.signers.length} · ${w.network} · ${w.signers.map((s) => s.name).join(', ')}'),
        const SizedBox(height: 16),
        _backupCard(theme),
        const SizedBox(height: 16),
        if (loading) const LinearProgressIndicator(),
        if (error != null) Text(error!, style: TextStyle(color: theme.colorScheme.error)),
        if (s != null) ...[
          Text(formatBtc(s.balanceSats.toInt()), style: theme.textTheme.displaySmall),
          Text('${formatSats(s.balanceSats.toInt())} · ${s.nCoins} coin(s) · ${formatSats(s.confirmedSats.toInt())} confirmed'),
          const SizedBox(height: 24),
          Text('Receive', style: theme.textTheme.titleMedium),
          const SizedBox(height: 8),
          Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Container(
              color: Colors.white,
              padding: const EdgeInsets.all(8),
              child: QrImageView(data: s.receiveAddress.toUpperCase(), size: 180),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                SelectableText(groupAddress(s.receiveAddress), style: const TextStyle(fontFamily: 'monospace', fontSize: 16)),
                TextButton.icon(
                  onPressed: () => Clipboard.setData(ClipboardData(text: s.receiveAddress)),
                  icon: const Icon(Icons.copy),
                  label: const Text('Copy address'),
                ),
                Text('QR contains: ${s.receiveAddress.toUpperCase()}',
                    style: theme.textTheme.bodySmall?.copyWith(fontFamily: 'monospace')),
              ]),
            ),
          ]),
        ],
        const Divider(height: 48),
        Text('Send', style: theme.textTheme.titleMedium),
        const SizedBox(height: 8),
        TextField(controller: to, decoration: const InputDecoration(labelText: 'Recipient address')),
        const SizedBox(height: 8),
        Row(children: [
          Expanded(
            child: TextField(
              controller: amount,
              enabled: !sendMax,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(labelText: 'Amount (sats)'),
            ),
          ),
          const SizedBox(width: 12),
          Row(children: [
            Checkbox(value: sendMax, onChanged: (v) => setState(() => sendMax = v ?? false)),
            const Text('Send everything'),
          ]),
        ]),
        const SizedBox(height: 8),
        Row(children: [
          SizedBox(
            width: 180,
            child: TextField(controller: feeRate, keyboardType: TextInputType.number, decoration: const InputDecoration(labelText: 'Fee rate (sat/vB)')),
          ),
          TextButton(onPressed: _suggestFee, child: const Text('Suggest')),
        ]),
        const SizedBox(height: 16),
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.icon(
            onPressed: busy || s == null ? null : _send,
            icon: busy ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)) : const Icon(Icons.send),
            label: const Text('Review & sign'),
          ),
        ),
      ]),
    );
  }
}

class _ConfirmSendDialog extends StatefulWidget {
  const _ConfirmSendDialog({required this.wallet, required this.preview});
  final KeyView wallet;
  final SendPreview preview;
  @override
  State<_ConfirmSendDialog> createState() => _ConfirmSendDialogState();
}

class _ConfirmSendDialogState extends State<_ConfirmSendDialog> {
  late final Set<String> chosen = {for (final s in widget.wallet.signers.take(widget.wallet.threshold)) s.deviceId};

  @override
  Widget build(BuildContext context) {
    final p = widget.preview;
    final ok = chosen.length >= widget.wallet.threshold;
    return AlertDialog(
      title: const Text('Confirm payment'),
      content: SizedBox(
        width: 480,
        child: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
            TxCard(
              outputs: [
                TxOutputRow(address: p.recipient, sats: p.amountSats.toInt()),
                if (p.changeSats != null) TxOutputRow(address: 'change (back to this wallet)', sats: p.changeSats!.toInt(), ownLabel: 'Change'),
              ],
              feeSats: p.feeSats.toInt(),
              feeRate: p.feeRate,
              nInputs: p.nInputs,
              verified: true,
            ),
            const SizedBox(height: 12),
            Text('Which signers will sign? (need ${widget.wallet.threshold})', style: Theme.of(context).textTheme.labelLarge),
            for (final s in widget.wallet.signers)
              CheckboxListTile(
                dense: true,
                title: Text(s.name),
                value: chosen.contains(s.deviceId),
                onChanged: (v) => setState(() => v == true ? chosen.add(s.deviceId) : chosen.remove(s.deviceId)),
              ),
          ]),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(onPressed: ok ? () => Navigator.pop(context, chosen.toList()) : null, child: const Text('Start signing')),
      ],
    );
  }
}
