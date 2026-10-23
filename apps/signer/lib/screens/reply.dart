import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';

/// Shows QR frames for the coordinator to scan.
class ReplyScreen extends StatelessWidget {
  const ReplyScreen({super.key, required this.frames, required this.title, required this.what});
  final List<String> frames;
  final String title;
  final String what;

  @override
  Widget build(BuildContext context) {
    final size = MediaQuery.of(context).size.shortestSide - 64;
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: ListView(padding: const EdgeInsets.all(16), children: [
        Center(child: AnimatedQr(frames: frames, size: size.clamp(200, 420), sound: const RustSoundPlayer())),
        const SizedBox(height: 16),
        Text(what, textAlign: TextAlign.center),
        const SizedBox(height: 8),
        const Text(
          'Hold the device steady until the coordinator has all parts.',
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 24),
        FilledButton(onPressed: () => Navigator.pop(context), child: const Text('Done')),
      ]),
    );
  }
}
