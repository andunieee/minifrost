import 'package:flutter/material.dart';

import '../src/rust/api.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key, required this.app});
  final CoordinatorApp app;
  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  final url = TextEditingController();
  String network = 'signet';
  String? saved;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    url.text = await widget.app.esploraUrl(network: network);
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(padding: const EdgeInsets.all(24), children: [
        Text('Esplora backend', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        const Text(
          'The coordinator is the only part of the system that goes online. It reads balances and '
          'publishes signed transactions through this Esplora server. Signers never see it.',
        ),
        const SizedBox(height: 16),
        TextField(
          controller: url,
          decoration: const InputDecoration(labelText: 'Base URL', hintText: 'https://mempool.space/api'),
        ),
        const SizedBox(height: 8),
        Row(children: [
          FilledButton(
            onPressed: () async {
              await widget.app.setEsploraUrl(url: url.text.trim());
              if (mounted) setState(() => saved = 'Saved');
            },
            child: const Text('Save'),
          ),
          const SizedBox(width: 12),
          TextButton(
            onPressed: () async {
              await widget.app.setEsploraUrl(url: null);
              await _load();
              if (mounted) setState(() => saved = 'Reset to default');
            },
            child: const Text('Use default'),
          ),
          if (saved != null) Padding(padding: const EdgeInsets.only(left: 12), child: Text(saved!)),
        ]),
        const SizedBox(height: 8),
        const Text('The URL applies to all wallets. Defaults per network: mempool.space (mainnet, testnet, signet).',
            style: TextStyle(fontSize: 12)),
      ]),
    );
  }
}
