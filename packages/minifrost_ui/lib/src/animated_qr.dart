import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';

import 'qr_scan_backend.dart';
import 'signer_color.dart';
import 'toast.dart';

/// Shows a list of text frames as a looping QR code. A single frame is shown still. With a
/// [sound] player it also offers to play the same frames as sound for a listening device.
class AnimatedQr extends StatefulWidget {
  const AnimatedQr({super.key, required this.frames, this.size = 320, this.fps = 4, this.sound, this.label, this.color});

  final List<String> frames;
  final double size;
  final double fps;
  final SoundPlayer? sound;

  /// What kind of message this code carries, shown above it.
  final String? label;

  /// Whose code this is: frames the code and tints its modules, so the scanning side can tell
  /// at a glance which device it is pointed at.
  final Color? color;

  @override
  State<AnimatedQr> createState() => _AnimatedQrState();
}

class _AnimatedQrState extends State<AnimatedQr> {
  int _i = 0;
  bool _showRaw = false;
  bool _playing = false;
  String? _soundError;
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
      if (_playing && old.frames != widget.frames) _play();
    }
  }

  Future<void> _toggleSound() async {
    if (_playing) {
      widget.sound?.stop();
      setState(() => _playing = false);
    } else {
      await _play();
    }
  }

  Future<void> _play() async {
    setState(() => _soundError = null);
    try {
      await widget.sound!.play(widget.frames);
      if (mounted) setState(() => _playing = true);
    } catch (e) {
      if (mounted) {
        setState(() {
          _playing = false;
          _soundError = '$e';
        });
      }
    }
  }

  void _restart() {
    _timer?.cancel();
    if (widget.frames.length > 1) {
      _timer = Timer.periodic(Duration(milliseconds: (1000 / widget.fps).round()), (_) {
        // the raw text lists every part, so the code holds still while it is being read
        if (mounted && !_showRaw) setState(() => _i = (_i + 1) % widget.frames.length);
      });
    }
  }

  @override
  void dispose() {
    if (_playing) widget.sound?.stop();
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.frames.isEmpty) return SizedBox(width: widget.size, height: widget.size);
    final i = _i % widget.frames.length;
    final color = widget.color;
    final ink = color == null ? Colors.black : qrInk(color);
    return Column(mainAxisSize: MainAxisSize.min, children: [
      if (widget.label != null) ...[
        QrKindLabel(widget.label!),
        const SizedBox(height: 8),
      ],
      Container(
        decoration: BoxDecoration(
          color: Colors.white, // QR needs a light quiet zone in dark mode too
          border: color == null ? null : Border.all(color: color, width: 8),
          borderRadius: color == null ? null : BorderRadius.circular(16),
        ),
        padding: const EdgeInsets.all(12),
        child: QrImageView(
          data: widget.frames[i],
          size: widget.size,
          errorCorrectionLevel: QrErrorCorrectLevel.M,
          backgroundColor: Colors.white,
          eyeStyle: QrEyeStyle(eyeShape: QrEyeShape.square, color: ink),
          dataModuleStyle: QrDataModuleStyle(dataModuleShape: QrDataModuleShape.square, color: ink),
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
                content: Text('Copied QR contents')));
          },
          icon: const Icon(Icons.copy),
          label: const Text('Copy contents'),
        ),
        if (widget.sound != null)
          TextButton.icon(
            onPressed: _toggleSound,
            icon: Icon(_playing ? Icons.stop : Icons.volume_up),
            label: Text(_playing ? 'Stop sound' : 'Play sound'),
          ),
        TextButton.icon(
          onPressed: () => setState(() => _showRaw = !_showRaw),
          icon: Icon(_showRaw ? Icons.visibility_off : Icons.visibility),
          label: Text(_showRaw ? 'Hide raw' : 'Show raw'),
        ),
      ]),
      if (_playing)
        const Padding(
          padding: EdgeInsets.only(top: 4),
          child: Text('Playing… the other device should be listening, close by.'),
        ),
      if (_soundError != null)
        Padding(
          padding: const EdgeInsets.only(top: 4),
          child: Text(_soundError!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
        ),
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
