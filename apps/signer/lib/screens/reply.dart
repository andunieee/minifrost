import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';
import 'scan.dart';

/// Shows QR frames for the coordinator to scan. With [next] set, it offers to scan the
/// coordinator's following code and pops with that payload; otherwise it pops with null.
class ReplyScreen extends StatelessWidget {
  const ReplyScreen({super.key, required this.frames, required this.title, required this.what, required this.deviceId, this.next});
  final List<String> frames;
  final String title;
  final String what;

  /// Title of the coordinator code that follows this reply.
  final String? next;

  /// This signer's id, so the code is framed in its colour.
  final String deviceId;

  @override
  Widget build(BuildContext context) {
    final size = MediaQuery.of(context).size.shortestSide - 64;
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: ListView(padding: const EdgeInsets.all(16), children: [
        Center(
          child: AnimatedQr(
            frames: frames,
            label: 'Showing: $what',
            size: size.clamp(200, 420),
            sound: const RustSoundPlayer(),
            color: signerColor(deviceId, Theme.of(context).brightness),
          ),
        ),
        const SizedBox(height: 16),
        Text(what, textAlign: TextAlign.center),
        const SizedBox(height: 8),
        const Text(
          'Hold the device steady until the coordinator has all parts.',
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 24),
        if (next case final next?) ...[
          Text('Once the coordinator has scanned every signer it shows “$next”.', textAlign: TextAlign.center),
          const SizedBox(height: 8),
          FilledButton.icon(
            icon: const Icon(Icons.qr_code_scanner),
            label: const Text('Scan next code'),
            onPressed: () async {
              // scan on top of this screen, so backing out of the scanner shows the reply again
              final payload = await Navigator.push<Uint8List>(context, MaterialPageRoute(builder: (_) => ScanScreen(expecting: next)));
              if (payload != null && context.mounted) Navigator.pop(context, payload);
            },
          ),
          const SizedBox(height: 8),
          OutlinedButton(onPressed: () => Navigator.pop(context), child: const Text('Back to home')),
        ] else
          FilledButton(onPressed: () => Navigator.pop(context), child: const Text('Done')),
      ]),
    );
  }
}
