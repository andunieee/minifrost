import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'src/rust/api.dart';
import 'src/rust/api/camera.dart';

/// Webcam preview + QR frame assembly. Calls [onPayload] once per completed animated code.
class ScanPanel extends StatefulWidget {
  const ScanPanel({super.key, required this.onPayload, this.hint = 'Hold the signer’s screen up to the camera'});
  final Future<void> Function(Uint8List payload) onPayload;
  final String hint;

  @override
  State<ScanPanel> createState() => _ScanPanelState();
}

class _ScanPanelState extends State<ScanPanel> {
  List<CameraInfo>? cameras;
  int? selected;
  StreamSubscription<CameraEvent>? sub;
  Uint8List? preview;
  QrScanner? scanner;
  int received = 0, total = 0;
  String? problem;
  String? error;
  bool handling = false;
  final paste = TextEditingController();

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    scanner = await newQrScanner();
    final cams = await listCameras();
    if (!mounted) return;
    setState(() => cameras = cams);
    if (cams.isNotEmpty) _start(cams.first.index);
  }

  void _start(int index) {
    sub?.cancel();
    setState(() {
      selected = index;
      error = null;
    });
    sub = startCamera(index: index).listen(
      (e) {
        switch (e) {
          case CameraEvent_Preview(:final field0):
            if (mounted) setState(() => preview = field0);
          case CameraEvent_Qr(:final field0):
            _push(field0);
        }
      },
      onError: (Object err) {
        if (mounted) setState(() => error = '$err');
      },
    );
  }

  Future<void> _push(String text) async {
    if (handling) return;
    final p = await scanner!.push(text: text);
    if (!mounted) return;
    if (p.payload != null) {
      handling = true;
      setState(() {
        received = 0;
        total = 0;
        problem = null;
      });
      try {
        await widget.onPayload(p.payload!);
      } finally {
        handling = false;
      }
    } else {
      setState(() {
        problem = p.problem;
        if (p.problem == null) {
          received = p.received;
          total = p.total;
        }
      });
    }
  }

  @override
  void dispose() {
    sub?.cancel();
    stopCamera();
    paste.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final cams = cameras;
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      if (cams == null) const Center(child: CircularProgressIndicator()),
      if (cams != null && cams.isEmpty)
        const Card(child: Padding(padding: EdgeInsets.all(16), child: Text('No camera found (/dev/video*). You can paste QR text below instead.'))),
      if (cams != null && cams.length > 1)
        DropdownButton<int>(
          value: selected,
          isExpanded: true,
          items: [for (final c in cams) DropdownMenuItem(value: c.index, child: Text(c.name))],
          onChanged: (i) => i == null ? null : _start(i),
        ),
      AspectRatio(
        aspectRatio: 4 / 3,
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: Container(
            color: Colors.black,
            child: preview == null
                ? const Center(child: Icon(Icons.videocam_off, color: Colors.white54, size: 48))
                : Image.memory(preview!, gaplessPlayback: true, fit: BoxFit.cover),
          ),
        ),
      ),
      const SizedBox(height: 8),
      if (error != null) Text(error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
      Text(problem ?? widget.hint, textAlign: TextAlign.center),
      if (total > 1) ...[
        const SizedBox(height: 8),
        LinearProgressIndicator(value: received / total),
        Text('$received of $total parts', textAlign: TextAlign.center),
      ],
      ExpansionTile(
        title: const Text('Paste QR text instead'),
        children: [
          TextField(controller: paste, minLines: 1, maxLines: 3, decoration: const InputDecoration(hintText: 'MF1:…')),
          Align(
            alignment: Alignment.centerRight,
            child: TextButton(
              onPressed: () {
                for (final line in paste.text.split(RegExp(r'\s+'))) {
                  if (line.isNotEmpty) _push(line);
                }
                paste.clear();
              },
              child: const Text('Add'),
            ),
          ),
        ],
      ),
    ]);
  }
}
