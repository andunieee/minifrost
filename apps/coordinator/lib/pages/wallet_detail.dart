import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:minifrost_ui/minifrost_ui.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../src/rust/api.dart';
import '../src/rust/engine.dart';
import 'ceremony.dart';

class WalletDetailPage extends StatefulWidget {
  const WalletDetailPage({super.key, required this.app, required this.wallet});
  final CoordinatorApp app;
  final KeyView wallet;
  @override
  State<WalletDetailPage> createState() => _WalletDetailPageState();
}

class _WalletDetailPageState extends State<WalletDetailPage> {
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
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() {
      loading = true;
      error = null;
    });
    try {
      final s = await widget.app.refreshWallet(keyId: widget.wallet.keyId);
      if (mounted) setState(() => summary = s);
    } catch (e) {
      if (mounted) setState(() => error = '$e');
    } finally {
      if (mounted) setState(() => loading = false);
    }
  }

  Future<void> _suggestFee() async {
    try {
      final r = await widget.app.suggestedFeeRate(keyId: widget.wallet.keyId, blocks: 6);
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
        keyId: widget.wallet.keyId,
        toAddress: to.text.trim(),
        amountSats: sendMax ? null : sats,
        feeRate: rate,
      );
      if (!mounted) return;
      final chosen = await showDialog<List<String>>(
        context: context,
        builder: (_) => _ConfirmSendDialog(wallet: widget.wallet, preview: preview),
      );
      if (chosen == null || !mounted) return;
      final update = await widget.app.beginSigning(keyId: widget.wallet.keyId, deviceIds: chosen);
      if (!mounted) return;
      await Navigator.push(
        context,
        MaterialPageRoute(
          builder: (_) => CeremonyPage(
            app: widget.app,
            title: 'Sign payment',
            initial: update,
            keyId: widget.wallet.keyId,
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
    final w = widget.wallet;
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
