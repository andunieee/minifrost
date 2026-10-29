import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import 'controller.dart';
import 'screens/home.dart';
import 'screens/setup.dart';
import 'src/rust/api.dart';
import 'vault.dart';

class SignerRoot extends StatefulWidget {
  const SignerRoot({super.key});

  @override
  State<SignerRoot> createState() => _SignerRootState();
}

class _SignerRootState extends State<SignerRoot> {
  /// Set once a signer exists; from then on the whole app wears that signer's colour.
  String? deviceId;

  @override
  Widget build(BuildContext context) {
    final id = deviceId;
    return MaterialApp(
      title: 'Minifrost Signer',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      theme: id == null ? ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true) : signerTheme(id, Brightness.light),
      darkTheme: id == null
          ? ThemeData(colorSchemeSeed: Colors.indigo, brightness: Brightness.dark, useMaterial3: true)
          : signerTheme(id, Brightness.dark),
      home: _Boot(onSigner: (id) => setState(() => deviceId = id)),
    );
  }
}

/// Decides between first-run setup and the unlocked signer.
class _Boot extends StatefulWidget {
  const _Boot({required this.onSigner});
  final void Function(String deviceId) onSigner;
  @override
  State<_Boot> createState() => _BootState();
}

class _BootState extends State<_Boot> {
  final Vault vault = createVault();
  SignerController? controller;
  VaultStatus? status;
  String? error;

  @override
  void initState() {
    super.initState();
    _start();
  }

  Future<void> _start() async {
    setState(() => error = null);
    try {
      final st = await vault.status();
      setState(() => status = st);
      if (st.exists) {
        final bytes = await vault.load();
        if (bytes == null) throw VaultException('missing', 'Vault file disappeared.');
        final signer = await loadSigner(snapshot: bytes);
        final c = SignerController(signer, vault, await signer.deviceId());
        await c.refresh();
        _ready(c);
      }
    } catch (e) {
      setState(() => error = e.toString());
    }
  }

  void _ready(SignerController c) {
    setState(() => controller = c);
    widget.onSigner(c.deviceId);
  }

  @override
  Widget build(BuildContext context) {
    final st = status;
    if (controller != null) return Home(controller: controller!);
    if (error != null) {
      return Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(mainAxisSize: MainAxisSize.min, children: [
              const Icon(Icons.lock_outline, size: 48),
              const SizedBox(height: 16),
              Text(error!, textAlign: TextAlign.center),
              const SizedBox(height: 16),
              FilledButton(onPressed: _start, child: const Text('Try again')),
            ]),
          ),
        ),
      );
    }
    if (st == null) return const Scaffold(body: Center(child: CircularProgressIndicator()));
    return Setup(vault: vault, onCreated: _ready);
  }
}
