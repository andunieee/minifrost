import 'package:flutter/material.dart';

/// A text field plus "Add" button for entering QR text by hand when scanning isn't possible.
/// Every `MF1:…` frame found in the text is passed to [onText], one at a time, in order.
class PasteBar extends StatefulWidget {
  const PasteBar({super.key, required this.onText});
  final Future<void> Function(String text) onText;

  @override
  State<PasteBar> createState() => _PasteBarState();
}

class _PasteBarState extends State<PasteBar> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _add() async {
    // frames are upper-case, but tolerate pasted text that was lower-cased or run together
    final frames = RegExp(r'MF1:[A-Z0-9:]+', caseSensitive: false)
        .allMatches(_controller.text)
        .map((m) => m.group(0)!.toUpperCase())
        .toList();
    if (frames.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('No QR frames (MF1:…) found in that text.')));
      return;
    }
    _controller.clear();
    for (final f in frames) {
      await widget.onText(f);
    }
  }

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Expanded(
            child: TextField(
              controller: _controller,
              minLines: 1,
              maxLines: 3,
              decoration: const InputDecoration(labelText: 'Paste QR text', hintText: 'MF1:…', border: OutlineInputBorder()),
            ),
          ),
          const SizedBox(width: 8),
          TextButton(onPressed: _add, child: const Text('Add')),
        ]),
      ),
    );
  }
}
