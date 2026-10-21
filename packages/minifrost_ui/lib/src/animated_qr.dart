import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';

/// Shows a list of text frames as a looping QR code. A single frame is shown still.
class AnimatedQr extends StatefulWidget {
  const AnimatedQr({super.key, required this.frames, this.size = 320, this.fps = 4});

  final List<String> frames;
  final double size;
  final double fps;

  @override
  State<AnimatedQr> createState() => _AnimatedQrState();
}

class _AnimatedQrState extends State<AnimatedQr> {
  int _i = 0;
  bool _showRaw = false;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _restart();
  }

  @override
  void didUpdateWidget(AnimatedQr old) {
    super.didUpdateWidget(old);
    if (old.frames != widget.frames || old.fps != widget.fps) {
      _i = 0;
      _restart();
    }
  }

  void _restart() {
    _timer?.cancel();
    if (widget.frames.length > 1) {
      _timer = Timer.periodic(Duration(milliseconds: (1000 / widget.fps).round()), (_) {
        if (mounted) setState(() => _i = (_i + 1) % widget.frames.length);
      });
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.frames.isEmpty) return SizedBox(width: widget.size, height: widget.size);
    final i = _i % widget.frames.length;
    return Column(mainAxisSize: MainAxisSize.min, children: [
      Container(
        color: Colors.white, // QR needs a light quiet zone in dark mode too
        padding: const EdgeInsets.all(12),
        child: QrImageView(
          data: widget.frames[i],
          size: widget.size,
          errorCorrectionLevel: QrErrorCorrectLevel.M,
          backgroundColor: Colors.white,
          gapless: false,
        ),
      ),
      if (widget.frames.length > 1)
        Padding(
          padding: const EdgeInsets.only(top: 8),
          child: Text('part ${i + 1} of ${widget.frames.length}',
              style: Theme.of(context).textTheme.bodySmall),
        ),
      Wrap(alignment: WrapAlignment.center, children: [
        TextButton.icon(
          onPressed: () async {
            await Clipboard.setData(ClipboardData(text: widget.frames.join('\n')));
            if (!context.mounted) return;
            ScaffoldMessenger.maybeOf(context)?.showSnackBar(SnackBar(
                content: Text(widget.frames.length > 1
                    ? 'Copied all ${widget.frames.length} parts'
                    : 'Copied QR contents')));
          },
          icon: const Icon(Icons.copy),
          label: Text(widget.frames.length > 1 ? 'Copy all parts' : 'Copy contents'),
        ),
        TextButton.icon(
          onPressed: () => setState(() => _showRaw = !_showRaw),
          icon: Icon(_showRaw ? Icons.visibility_off : Icons.visibility),
          label: Text(_showRaw ? 'Hide raw' : 'Show raw'),
        ),
      ]),
      if (_showRaw)
        ConstrainedBox(
          constraints: BoxConstraints(maxWidth: widget.size + 24, maxHeight: 200),
          child: SingleChildScrollView(
            child: SelectableText(widget.frames.join('\n'),
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12)),
          ),
        ),
    ]);
  }
}
