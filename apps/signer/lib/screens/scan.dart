import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';

/// Scans an animated QR code (webcam on desktop, phone camera on Android) or takes pasted QR
/// text. Pops with the completed payload.
class ScanScreen extends StatefulWidget {
  const ScanScreen({
    super.key,
    this.hint = 'Point the camera at the coordinator’s QR code',
    this.expecting = 'coordinator message (key generation, transaction, backup request…)',
  });
  final String hint;
  final String expecting;

  @override
  State<ScanScreen> createState() => _ScanScreenState();
}

class _ScanScreenState extends State<ScanScreen> {
  /// The camera can read the same code again while this screen is still animating out. Only the
  /// first payload may pop: a second pop would close the screen that replaces this one (the
  /// acceptance prompt, or the reply QR after it).
  bool delivered = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Scan')),
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 640),
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(12),
              child: ScanPanel(
                backend: const RustQrBackend(),
                hint: widget.hint,
                expecting: widget.expecting,
                onPayload: (bytes) async {
                  if (delivered) return;
                  delivered = true;
                  Navigator.pop<Uint8List>(context, bytes);
                },
              ),
            ),
          ),
        ),
      ),
    );
  }
}
