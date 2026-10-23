import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';

/// Scans an animated QR code (webcam on desktop, phone camera on Android) or takes pasted QR
/// text. Pops with the completed payload.
class ScanScreen extends StatelessWidget {
  const ScanScreen({super.key, this.hint = 'Point the camera at the coordinator’s QR code'});
  final String hint;

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
                hint: hint,
                onPayload: (bytes) async => Navigator.pop<Uint8List>(context, bytes),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
