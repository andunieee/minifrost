import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../src/rust/api.dart';
import '../src/rust/engine.dart';
import '../src/rust/engine/restore.dart';
import 'ceremony.dart';
import 'pick_signer.dart';

/// Rebuild a wallet from at least a threshold of shares, the way frostsnap does: each share comes
/// either from a physical backup typed into a signer, or from a signer that still holds it.
class RestorePage extends StatefulWidget {
  const RestorePage({super.key, required this.app});
  final CoordinatorApp app;
  @override
  State<RestorePage> createState() => _RestorePageState();
}

class _RestorePageState extends State<RestorePage> {
  late Future<(List<RestorationView>, List<PendingConsolidationView>)> data = _load();

  Future<(List<RestorationView>, List<PendingConsolidationView>)> _load() async =>
      (await widget.app.restorations(), await widget.app.pendingConsolidations());

  void _reload() => setState(() {
    data = _load();
  });

  Future<Finished?> _ceremony(String title, Update update) {
    return Navigator.push<Finished?>(
      context,
      MaterialPageRoute(builder: (_) => CeremonyPage(app: widget.app, title: title, initial: update)),
    );
  }

  /// Run [action] and report its error as a snackbar; reloads afterwards either way.
  Future<void> _guard(Future<void> Function() action) async {
    try {
      await action();
    } catch (e) {
      if (mounted) toast(context, e);
    }
    if (mounted) _reload();
  }

  Future<void> _startFromBackup() => _guard(() async {
        final signers = await widget.app.signers();
        if (!mounted) return;
        final form = await showDialog<_NewRestoration>(
          context: context,
          builder: (_) => _NewRestorationDialog(signers: signers),
        );
        if (form == null || !mounted) return;
        final update = await widget.app.startEnterBackupNewWallet(
          deviceId: form.signer.deviceId,
          name: form.name,
          network: form.network,
          threshold: form.threshold,
        );
        if (!mounted) return;
        await _ceremony('Restore “${form.name}”', update);
      });

  /// Ask a signer what it holds; returns the chosen share, or null.
  Future<HeldShareView?> _askSignerForShare({String? title}) async {
    final signers = await widget.app.signers();
    if (!mounted) return null;
    final signer = await pickSigner(
      context,
      signers: signers,
      title: title ?? 'Which signer holds a share?',
      hint: 'The signer tells the coordinator which shares it holds. Nothing secret is sent.',
    );
    if (signer == null || !mounted) return null;
    final update = await widget.app.startRequestHeldShares(deviceId: signer.deviceId);
    if (!mounted) return null;
    final result = await _ceremony('Shares held by ${signer.name}', update);
    if (result is! Finished_HeldShares || !mounted) return null;
    if (result.shares.isEmpty) {
      toast(context, '${signer.name} holds no shares.');
      return null;
    }
    return showDialog<HeldShareView>(context: context, builder: (_) => _HeldShareDialog(shares: result.shares));
  }

  Future<void> _startFromSigner() => _guard(() async {
        final share = await _askSignerForShare();
        if (share == null) return;
        await widget.app.startRestorationFromHeldShare(deviceId: share.deviceId, shareIndex: share.shareIndex);
      });

  Future<void> _addFromBackup(RestorationView r) => _guard(() async {
        final signers = await widget.app.signers();
        if (!mounted) return;
        final signer = await pickSigner(
          context,
          signers: signers,
          title: 'Which signer will take the backup?',
          hint: 'Use a different signer for each backup.',
        );
        if (signer == null || !mounted) return;
        final update = await widget.app.startEnterBackupRestoration(
          deviceId: signer.deviceId,
          restorationId: r.restorationId,
        );
        if (!mounted) return;
        await _ceremony('Add a backup to “${r.name}”', update);
      });

  Future<void> _addFromSigner(RestorationView r) => _guard(() async {
        final share = await _askSignerForShare();
        if (share == null) return;
        await widget.app.addHeldShareToRestoration(
          restorationId: r.restorationId,
          deviceId: share.deviceId,
          shareIndex: share.shareIndex,
        );
      });

  Future<void> _finish(RestorationView r) => _guard(() async {
        await widget.app.finishRestoration(restorationId: r.restorationId);
        if (!mounted) return;
        toast(context, '“${r.name}” was restored. Signers restored from a backup still need to be finished below.');
      });

  Future<void> _cancel(RestorationView r) => _guard(() async {
        final ok = await showDialog<bool>(
          context: context,
          builder: (_) => AlertDialog(
            title: Text('Cancel restoring “${r.name}”?'),
            content: const Text('The shares gathered so far are forgotten by this coordinator. Backups saved on signers stay there.'),
            actions: [
              TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Keep it')),
              FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Cancel restoration')),
            ],
          ),
        );
        if (ok == true) await widget.app.cancelRestoration(restorationId: r.restorationId);
      });

  Future<void> _consolidate(PendingConsolidationView p) => _guard(() async {
        final update = await widget.app.startConsolidate(keyId: p.keyId, deviceId: p.deviceId);
        if (!mounted) return;
        await _ceremony('Finish ${p.deviceName}', update);
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
          final (restorations, pending) = d;
          return ListView(padding: const EdgeInsets.all(16), children: [
            Text(
              'Rebuild a wallet from at least a threshold of its shares. A share comes from a backup '
              '(25 words) typed into a signer, or from a signer that still holds it.',
              style: theme.textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Wrap(spacing: 8, runSpacing: 8, children: [
              FilledButton.icon(
                onPressed: _startFromBackup,
                icon: const Icon(Icons.edit_note),
                label: const Text('Restore from a backup'),
              ),
              OutlinedButton.icon(
                onPressed: _startFromSigner,
                icon: const Icon(Icons.phone_android),
                label: const Text('Restore from a signer'),
              ),
            ]),
            if (restorations.isNotEmpty) ...[
              const SizedBox(height: 24),
              Text('In progress', style: theme.textTheme.titleMedium),
              for (final r in restorations)
                _RestorationCard(
                  view: r,
                  onAddBackup: () => _addFromBackup(r),
                  onAddFromSigner: () => _addFromSigner(r),
                  onFinish: r.restorable ? () => _finish(r) : null,
                  onCancel: () => _cancel(r),
                  onRemoveShare: (s) => _guard(() async {
                    await widget.app.deleteRestorationShare(restorationId: r.restorationId, deviceId: s.deviceId);
                  }),
                ),
            ],
            if (pending.isNotEmpty) ...[
              const SizedBox(height: 24),
              Text('Signers waiting to be finished', style: theme.textTheme.titleMedium),
              const SizedBox(height: 4),
              const Text(
                'These signers took a backup. Finishing turns it into their normal share of the wallet; '
                'until then they cannot sign.',
              ),
              for (final p in pending)
                Card(
                  child: ListTile(
                    leading: const Icon(Icons.hourglass_bottom),
                    title: Text('${p.deviceName} · key #${p.shareIndex}'),
                    subtitle: Text('wallet “${p.keyName}”'),
                    trailing: FilledButton(onPressed: () => _consolidate(p), child: const Text('Finish')),
                  ),
                ),
            ],
            if (restorations.isEmpty && pending.isEmpty)
              const Padding(padding: EdgeInsets.only(top: 48), child: Center(child: Text('Nothing is being restored.'))),
          ]);
        },
      ),
    );
  }
}

class _RestorationCard extends StatelessWidget {
  const _RestorationCard({
    required this.view,
    required this.onAddBackup,
    required this.onAddFromSigner,
    required this.onFinish,
    required this.onCancel,
    required this.onRemoveShare,
  });
  final RestorationView view;
  final VoidCallback onAddBackup;
  final VoidCallback onAddFromSigner;
  final VoidCallback? onFinish;
  final VoidCallback onCancel;
  final void Function(RestorationShareView) onRemoveShare;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final got = view.got;
    final needed = view.threshold;
    final progress = [
      if (got != null) '$got' else 'some',
      'of ${needed ?? 'an unknown number of'} shares gathered',
      if (view.incompatible > 0) ' · ${view.incompatible} do not fit',
    ].join(' ');
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('${view.name} · ${networkLabel(view.network)}', style: theme.textTheme.titleMedium),
          const SizedBox(height: 4),
          Text(progress),
          if (needed != null && got != null) ...[
            const SizedBox(height: 8),
            LinearProgressIndicator(value: (got / needed).clamp(0, 1).toDouble()),
          ],
          if (view.restorable)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text('Enough shares to rebuild the wallet.', style: TextStyle(color: Colors.green.shade700)),
            ),
          const SizedBox(height: 8),
          for (final s in view.shares)
            ListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              leading: Icon(
                s.compatibility == 'incompatible' ? Icons.error_outline : Icons.vpn_key_outlined,
                color: s.compatibility == 'incompatible' ? theme.colorScheme.error : null,
              ),
              title: Text('Key #${s.shareIndex} on ${s.deviceName}'),
              subtitle: Text(switch (s.compatibility) {
                'compatible' => 'fits',
                'incompatible' => 'does not fit the other shares',
                _ => 'not checked yet',
              }),
              trailing: IconButton(
                icon: const Icon(Icons.close),
                tooltip: 'Remove this share',
                onPressed: () => onRemoveShare(s),
              ),
            ),
          const SizedBox(height: 8),
          Wrap(spacing: 8, runSpacing: 8, children: [
            OutlinedButton.icon(onPressed: onAddBackup, icon: const Icon(Icons.edit_note), label: const Text('Add a backup')),
            OutlinedButton.icon(
              onPressed: onAddFromSigner,
              icon: const Icon(Icons.phone_android),
              label: const Text('Add a signer’s share'),
            ),
            FilledButton.icon(onPressed: onFinish, icon: const Icon(Icons.check), label: const Text('Restore wallet')),
            TextButton(onPressed: onCancel, child: const Text('Cancel')),
          ]),
        ]),
      ),
    );
  }
}

class _NewRestoration {
  _NewRestoration(this.name, this.network, this.threshold, this.signer);
  final String name;
  final String network;
  final int? threshold;
  final SignerInfo signer;
}

class _NewRestorationDialog extends StatefulWidget {
  const _NewRestorationDialog({required this.signers});
  final List<SignerInfo> signers;
  @override
  State<_NewRestorationDialog> createState() => _NewRestorationDialogState();
}

class _NewRestorationDialogState extends State<_NewRestorationDialog> {
  final name = TextEditingController();
  String network = 'signet';
  int? threshold;
  SignerInfo? signer;

  @override
  Widget build(BuildContext context) {
    final valid = name.text.trim().isNotEmpty && signer != null;
    return AlertDialog(
      title: const Text('Restore from a backup'),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
            const Text('Use the wallet name, network and threshold from your backup sheet.'),
            const SizedBox(height: 12),
            TextField(
              controller: name,
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
            const SizedBox(height: 12),
            DropdownButtonFormField<int?>(
              initialValue: threshold,
              decoration: const InputDecoration(labelText: 'Signatures needed'),
              items: [
                const DropdownMenuItem<int?>(value: null, child: Text('I don’t know')),
                for (var i = 1; i <= 10; i++) DropdownMenuItem<int?>(value: i, child: Text('$i')),
              ],
              onChanged: (v) => setState(() => threshold = v),
            ),
            const SizedBox(height: 12),
            Text('Signer that takes the first backup', style: Theme.of(context).textTheme.labelLarge),
            if (widget.signers.isEmpty) const Text('No signers yet. Add the signer phone on the Signers page first.'),
            for (final s in widget.signers)
              ListTile(
                dense: true,
                selected: signer == s,
                leading: Icon(signer == s ? Icons.radio_button_checked : Icons.radio_button_unchecked),
                title: Text(s.name),
                onTap: () => setState(() => signer = s),
              ),
          ]),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(
          onPressed: valid ? () => Navigator.pop(context, _NewRestoration(name.text.trim(), network, threshold, signer!)) : null,
          child: const Text('Continue'),
        ),
      ],
    );
  }
}

class _HeldShareDialog extends StatelessWidget {
  const _HeldShareDialog({required this.shares});
  final List<HeldShareView> shares;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Choose a share'),
      content: SizedBox(
        width: 460,
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          for (final s in shares)
            ListTile(
              leading: const Icon(Icons.vpn_key_outlined),
              title: Text('Key #${s.shareIndex}${s.keyName == null ? '' : ' of “${s.keyName}”'}'),
              subtitle: Text([
                if (s.threshold != null) '${s.threshold} signatures needed',
                if (s.network != null) networkLabel(s.network!),
                if (s.needsConsolidation) 'typed-in backup, not yet a normal share',
                if (s.knownWallet != null) 'already part of “${s.knownWallet}” here',
              ].join(' · ')),
              onTap: () => Navigator.pop(context, s),
            ),
        ]),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel'))],
    );
  }
}
