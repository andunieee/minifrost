import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../src/rust/api.dart';
import '../src/rust/api/camera.dart' as cam;
import 'paste_bar.dart';

/// Screens an animated QR code with the machine's webcam, for running the signer on the
/// desktop. Capture and QR detection happen in Rust (the signer crate's camera api on top of
/// the shared `desktop_camera` crate); the Rust side hands us preview JPEGs and decoded QR
/// texts as a stream. Pops with the completed payload.
class WebcamScanScreen extends StatefulWidget {
  const WebcamScanScreen({super.key, this.hint = 'Point the webcam at the coordinator’s QR code'});
  final String hint;

  @override
  State<WebcamScanScreen> createState() => _WebcamScanScreenState();
}

class _WebcamScanScreenState extends State<WebcamScanScreen> {
  QrScanner? _scanner;
  List<cam.CameraInfo> _cams = [];
  StreamSubscription<cam.CameraEvent>? _sub;
  Uint8List? _jpeg;
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
      final cams = await cam.listCameras(); // errors on non-desktop builds
      if (!mounted) return;
      if (cams.isEmpty) {
        setState(() => _error = 'No webcam found.');
        return;
      }
      setState(() => _cams = cams);
      await _start(cams.first);
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  Stream<cam.CameraEvent> _open(cam.CameraInfo info) {
    _received = 0;
    _total = 0;
    _problem = _error = null;
    return cam.startCamera(index: info.index); // also stops any previous stream
  }

  Future<void> _start(cam.CameraInfo info) async {
    final stream = _open(info);
    try {
      _sub = stream.listen(_onEvent, onError: (e) {
        if (mounted && !_done) setState(() => _error = 'Camera error: $e');
      });
    } catch (e) {
      if (mounted) setState(() => _error = 'Camera error: $e');
      return;
    }
    setState(() {});
  }

  void _onEvent(cam.CameraEvent event) {
    switch (event) {
      case cam.CameraEvent_Preview(:final field0):
        if (mounted && !_done) setState(() => _jpeg = field0);
      case cam.CameraEvent_Qr(:final field0):
        if (_busy || _done) return;
        _busy = true;
        _process(field0).whenComplete(() => _busy = false);
    }
  }

  Future<void> _process(String text) async {
    final p = await _scanner!.push(text: text);
    if (!mounted || _done) return;
    if (p.payload != null) {
      _done = true;
      await _sub!.cancel().catchError((_) {});
      cam.stopCamera();
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

  @override
  void dispose() {
    _sub?.cancel();
    cam.stopCamera();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final jpeg = _jpeg;
    return Scaffold(
      appBar: AppBar(title: const Text('Scan')),
      body: Column(children: [
        Expanded(
          child: _error != null
          ? Center(child: Padding(padding: const EdgeInsets.all(24), child: Text(_error!)))
          : jpeg == null
              ? const Center(child: CircularProgressIndicator())
              : Stack(fit: StackFit.expand, children: [
                  Image.memory(jpeg, fit: BoxFit.cover),
                  if (_cams.length > 1)
                    Positioned(
                      top: 8,
                      right: 8,
                      child: PopupMenuButton<cam.CameraInfo>(
                        icon: const Icon(Icons.photo_camera_outlined, color: Colors.white),
                        onSelected: (info) async {
                          await _sub?.cancel();
                          await _start(info);
                        },
                        itemBuilder: (_) => [
                          for (final c in _cams) PopupMenuItem(value: c, child: Text(c.name)),
                        ],
                      ),
                    ),
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
        ),
        PasteBar(onText: _process),
      ]),
    );
  }
}
