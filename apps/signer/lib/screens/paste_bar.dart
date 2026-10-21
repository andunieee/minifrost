import 'package:flutter/material.dart';

/// A text field plus "Add" button for entering QR text by hand when scanning isn't possible.
/// Each whitespace-separated token is passed to [onText].
class PasteBar extends StatefulWidget {
  const PasteBar({super.key, required this.onText});
  final void Function(String text) onText;

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

  void _add() {
    for (final t in _controller.text.split(RegExp(r'\s+'))) {
      if (t.isNotEmpty) widget.onText(t);
    }
    _controller.clear();
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
