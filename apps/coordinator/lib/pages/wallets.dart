import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../src/rust/api.dart';
import '../src/rust/engine.dart';
import 'ceremony.dart';
import 'wallet_detail.dart';

class WalletsPage extends StatefulWidget {
  const WalletsPage({super.key, required this.app});
  final CoordinatorApp app;
  @override
  State<WalletsPage> createState() => _WalletsPageState();
}

class _WalletsPageState extends State<WalletsPage> {
  late Future<List<KeyView>> keys = widget.app.keys();

  void _reload() => setState(() {
    keys = widget.app.keys();
  });

  Future<void> _create() async {
    final signers = await widget.app.signers();
    if (!mounted) return;
    if (signers.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Add signers first.')));
      return;
    }
    final form = await showDialog<_NewWallet>(context: context, builder: (_) => _NewWalletDialog(signers: signers));
    if (form == null || !mounted) return;
    try {
      final update = await widget.app.startKeygen(
        keyName: form.name,
        threshold: form.threshold,
        deviceIds: form.signerIds,
        network: form.network,
      );
      if (!mounted) return;
      await Navigator.push(
        context,
        MaterialPageRoute(builder: (_) => CeremonyPage(app: widget.app, title: 'Create “${form.name}”', initial: update)),
      );
    } catch (e) {
      if (mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
    }
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Wallets')),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _create,
        icon: const Icon(Icons.add),
        label: const Text('New wallet'),
      ),
      body: FutureBuilder(
        future: keys,
        builder: (context, snap) {
          final list = snap.data;
          if (list == null) return const Center(child: CircularProgressIndicator());
          if (list.isEmpty) {
            return const Center(child: Text('No wallets yet. Create one with your signers.'));
          }
          return ListView(padding: const EdgeInsets.all(16), children: [
            for (final k in list)
              Card(
                child: ListTile(
                  leading: const Icon(Icons.account_balance_wallet),
                  title: Text(k.name),
                  subtitle: Text('${k.threshold}-of-${k.signers.length} · ${networkLabel(k.network)} · ${k.signers.map((s) => s.name).join(', ')}'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () async {
                    await Navigator.push(context, MaterialPageRoute(builder: (_) => WalletDetailPage(app: widget.app, wallet: k)));
                    _reload();
                  },
                ),
              ),
          ]);
        },
      ),
    );
  }
}

class _NewWallet {
  _NewWallet(this.name, this.network, this.threshold, this.signerIds);
  final String name;
  final String network;
  final int threshold;
  final List<String> signerIds;
}

class _NewWalletDialog extends StatefulWidget {
  const _NewWalletDialog({required this.signers});
  final List<SignerInfo> signers;
  @override
  State<_NewWalletDialog> createState() => _NewWalletDialogState();
}

class _NewWalletDialogState extends State<_NewWalletDialog> {
  final name = TextEditingController();
  String network = 'signet';
  late final Set<String> chosen = {for (final s in widget.signers) s.deviceId};
  int? threshold;

  int get effectiveThreshold => (threshold ?? (chosen.length / 2).ceil()).clamp(1, chosen.isEmpty ? 1 : chosen.length);

  @override
  Widget build(BuildContext context) {
    final valid = name.text.trim().isNotEmpty && chosen.isNotEmpty;
    return AlertDialog(
      title: const Text('New wallet'),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
            TextField(controller: name, decoration: const InputDecoration(labelText: 'Wallet name'), onChanged: (_) => setState(() {})),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              initialValue: network,
              decoration: const InputDecoration(labelText: 'Network'),
              items: const [
                DropdownMenuItem(value: 'signet', child: Text('Mutinynet')),
                DropdownMenuItem(value: 'testnet', child: Text('Testnet')),
                DropdownMenuItem(value: 'bitcoin', child: Text('Bitcoin mainnet — real money')),
              ],
              onChanged: (v) => setState(() => network = v ?? network),
            ),
            const SizedBox(height: 12),
            Text('Signers', style: Theme.of(context).textTheme.labelLarge),
            for (final s in widget.signers)
              CheckboxListTile(
                dense: true,
                value: chosen.contains(s.deviceId),
                title: Text(s.name),
                onChanged: (v) => setState(() => v == true ? chosen.add(s.deviceId) : chosen.remove(s.deviceId)),
              ),
            const SizedBox(height: 8),
            Row(children: [
              const Text('Signatures needed:'),
              const SizedBox(width: 12),
              DropdownButton<int>(
                value: effectiveThreshold,
                items: [for (var i = 1; i <= (chosen.isEmpty ? 1 : chosen.length); i++) DropdownMenuItem(value: i, child: Text('$i of ${chosen.length}'))],
                onChanged: (v) => setState(() => threshold = v),
              ),
            ]),
          ]),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(
          onPressed: valid
              ? () => Navigator.pop(context, _NewWallet(name.text.trim(), network, effectiveThreshold, chosen.toList()))
              : null,
          child: const Text('Start'),
        ),
      ],
    );
  }
}
