import 'package:flutter/material.dart';

import '../controller.dart';
import '../src/rust/api.dart';
import '../vault.dart';

class Setup extends StatefulWidget {
  const Setup({super.key, required this.vault, required this.onCreated});
  final Vault vault;
  final void Function(SignerController) onCreated;

  @override
  State<Setup> createState() => _SetupState();
}

class _SetupState extends State<Setup> {
  final name = TextEditingController();
  bool busy = false;
  String? error;

  Future<void> _create() async {
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final signer = await newSigner(name: name.text.trim());
      // Store before doing anything else; this also creates the hardware-backed key.
      await widget.vault.save(await signer.snapshot());
      final c = SignerController(signer, widget.vault);
      await c.refresh();
      widget.onCreated(c);
    } catch (e) {
      setState(() {
        busy = false;
        error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Set up signer')),
      body: ListView(padding: const EdgeInsets.all(24), children: [
        const Text(
          'This phone becomes one signer of a threshold wallet. It never connects to the internet: '
          'it talks to the coordinator only through QR codes.',
        ),
        const SizedBox(height: 24),
        TextField(
          controller: name,
          maxLength: 40,
          decoration: const InputDecoration(labelText: 'Name of this signer', hintText: 'e.g. Alice'),
          onChanged: (_) => setState(() {}),
        ),
        const SizedBox(height: 16),
        if (error != null) Text(error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
        FilledButton(
          onPressed: busy || name.text.trim().isEmpty ? null : _create,
          child: busy ? const CircularProgressIndicator() : const Text('Create signer'),
        ),
      ]),
    );
  }
}
