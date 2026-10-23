import 'package:flutter/material.dart';

import '../controller.dart';
import '../src/rust/engine.dart';
import 'flow.dart';
import 'reply.dart';

class Home extends StatelessWidget {
  const Home({super.key, required this.controller});
  final SignerController controller;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) => Scaffold(
        appBar: AppBar(title: const Text('Minifrost Signer')),
        body: ListView(padding: const EdgeInsets.all(16), children: [
          FutureBuilder(
            future: Future.wait([controller.signer.name(), controller.signer.deviceId()]),
            builder: (context, snap) {
              final d = snap.data;
              return Card(
                child: ListTile(
                  leading: const Icon(Icons.shield_outlined),
                  title: Text(d?[0] ?? '…'),
                  subtitle: Text(d == null ? '' : 'id ${d[1].substring(0, 16)}…',
                      style: const TextStyle(fontFamily: 'monospace')),
                ),
              );
            },
          ),
          const SizedBox(height: 8),
          Text('Keys held by this signer', style: Theme.of(context).textTheme.titleMedium),
          if (controller.keys.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16),
              child: Text('None yet. Show your enrolment code to a coordinator, then scan its key generation codes.'),
            ),
          for (final k in controller.keys) _KeyTile(k),
          if (controller.pendingBackups.isNotEmpty) ...[
            const SizedBox(height: 16),
            Text('Restoring', style: Theme.of(context).textTheme.titleMedium),
            for (final b in controller.pendingBackups)
              Card(
                child: ListTile(
                  leading: const Icon(Icons.history),
                  title: Text('Backup #${b.shareIndex}${b.keyName == null ? '' : ' of “${b.keyName}”'}'),
                  subtitle: const Text('Saved on this signer. It becomes a normal share once the coordinator finishes restoring the wallet.'),
                  isThreeLine: true,
                ),
              ),
          ],
          const SizedBox(height: 16),
          OutlinedButton.icon(
            icon: const Icon(Icons.qr_code),
            label: const Text('Show enrolment code'),
            onPressed: () async {
              final frames = await controller.signer.helloFrames();
              if (!context.mounted) return;
              Navigator.push(
                context,
                MaterialPageRoute(
                  builder: (_) => ReplyScreen(
                    frames: frames,
                    title: 'Enrolment code',
                    what: 'Let the coordinator scan this to add you as a signer.',
                  ),
                ),
              );
            },
          ),
        ]),
        floatingActionButton: FloatingActionButton.extended(
          icon: const Icon(Icons.qr_code_scanner),
          label: const Text('Scan coordinator'),
          onPressed: () => runScanFlow(context, controller),
        ),
      ),
    );
  }
}

class _KeyTile extends StatelessWidget {
  const _KeyTile(this.k);
  final KeyInfo k;
  @override
  Widget build(BuildContext context) => Card(
        child: ListTile(
          leading: const Icon(Icons.vpn_key_outlined),
          title: Text(k.name),
          subtitle: Text('${k.threshold}-of-? · ${k.network}\n${k.keyId.substring(0, 16)}…'),
          isThreeLine: true,
        ),
      );
}
