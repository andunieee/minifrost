import 'package:flutter/material.dart';

import 'controller.dart';
import 'screens/home.dart';
import 'screens/setup.dart';
import 'src/rust/api.dart';
import 'vault.dart';

class SignerRoot extends StatelessWidget {
  const SignerRoot({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Minifrost Signer',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      theme: ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true),
      darkTheme: ThemeData(colorSchemeSeed: Colors.indigo, brightness: Brightness.dark, useMaterial3: true),
      home: const _Boot(),
    );
  }
}

/// Decides between first-run setup and the unlocked signer.
class _Boot extends StatefulWidget {
  const _Boot();
  @override
  State<_Boot> createState() => _BootState();
}

class _BootState extends State<_Boot> {
  final vault = Vault();
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
      if (!st.secureLockScreen) return; // the UI explains
      if (st.exists) {
        final bytes = await vault.load(); // asks for biometrics / screen lock
        if (bytes == null) throw VaultException('missing', 'Vault file disappeared.');
        final signer = await loadSigner(snapshot: bytes);
        final c = SignerController(signer, vault);
        await c.refresh();
        setState(() => controller = c);
      }
    } catch (e) {
      setState(() => error = e.toString());
    }
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
    if (!st.secureLockScreen) {
      return Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(mainAxisSize: MainAxisSize.min, children: [
              const Icon(Icons.screen_lock_portrait, size: 48),
              const SizedBox(height: 16),
              const Text(
                'Set a screen lock (PIN, pattern or password) first. Your key share is protected by it.',
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 16),
              FilledButton(onPressed: _start, child: const Text('I set one — continue')),
            ]),
          ),
        ),
      );
    }
    return Setup(
      vault: vault,
      onCreated: (c) => setState(() => controller = c),
    );
  }
}
