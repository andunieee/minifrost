import 'dart:typed_data';

import 'package:camera/camera.dart';
import 'package:flutter/material.dart';

import '../src/rust/api.dart';

/// Scans an animated QR code with the back camera. QR detection and frame reassembly are done in
/// Rust (the app has no Google/ML-Kit dependency). Pops with the completed payload.
class ScanScreen extends StatefulWidget {
  const ScanScreen({super.key, this.hint = 'Point the camera at the coordinator’s QR code'});
  final String hint;

  @override
  State<ScanScreen> createState() => _ScanScreenState();
}

class _ScanScreenState extends State<ScanScreen> {
  CameraController? _camera;
  QrScanner? _scanner;
  String? _error;
  bool _busy = false;
  bool _done = false;
  int _received = 0;
  int _total = 0;
  String? _problem;

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    try {
      _scanner = await newQrScanner();
      final cams = await availableCameras();
      final cam = cams.firstWhere((c) => c.lensDirection == CameraLensDirection.back, orElse: () => cams.first);
      final ctrl = CameraController(
        cam,
        ResolutionPreset.high,
        enableAudio: false,
        imageFormatGroup: ImageFormatGroup.yuv420,
      );
      await ctrl.initialize();
      if (!mounted) return;
      setState(() => _camera = ctrl);
      await ctrl.startImageStream(_onFrame);
    } on CameraException catch (e) {
      setState(() => _error = e.code == 'CameraAccessDenied'
          ? 'Camera permission is needed to read QR codes.'
          : 'Camera error: ${e.description ?? e.code}');
    } catch (e) {
      setState(() => _error = '$e');
    }
  }

  void _onFrame(CameraImage img) {
    if (_busy || _done) return;
    _busy = true;
    _process(img).whenComplete(() => _busy = false);
  }

  Future<void> _process(CameraImage img) async {
    final y = img.planes[0];
    final texts = await decodeQrLuma(
      width: img.width,
      height: img.height,
      rowStride: y.bytesPerRow,
      luma: y.bytes,
    );
    for (final t in texts) {
      final p = await _scanner!.push(text: t);
      if (!mounted || _done) return;
      if (p.payload != null) {
        _done = true;
        await _camera?.stopImageStream().catchError((_) {});
        if (mounted) Navigator.pop<Uint8List>(context, p.payload);
        return;
      }
      setState(() {
        if (p.problem != null) {
          _problem = p.problem;
        } else {
          _problem = null;
          _received = p.received;
          _total = p.total;
        }
      });
    }
  }

  @override
  void dispose() {
    _camera?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final cam = _camera;
    return Scaffold(
      appBar: AppBar(title: const Text('Scan')),
      body: _error != null
          ? Center(child: Padding(padding: const EdgeInsets.all(24), child: Text(_error!)))
          : cam == null
              ? const Center(child: CircularProgressIndicator())
              : Stack(fit: StackFit.expand, children: [
                  CameraPreview(cam),
                  Align(
                    alignment: Alignment.bottomCenter,
                    child: Container(
                      color: Colors.black54,
                      padding: const EdgeInsets.all(16),
                      width: double.infinity,
                      child: Column(mainAxisSize: MainAxisSize.min, children: [
                        Text(_problem ?? widget.hint, style: const TextStyle(color: Colors.white)),
                        if (_total > 1) ...[
                          const SizedBox(height: 8),
                          LinearProgressIndicator(value: _received / _total),
                          const SizedBox(height: 4),
                          Text('$_received of $_total parts', style: const TextStyle(color: Colors.white)),
                        ],
                      ]),
                    ),
                  ),
                ]),
    );
  }
}
