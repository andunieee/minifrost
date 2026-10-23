import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../controller.dart';
import '../src/rust/api.dart';

/// Type in a physical backup: the key number and the 25 words, in order. The signer checks the
/// words' own checksums; the coordinator then checks the share fits the wallet.
class EnterBackupScreen extends StatefulWidget {
  const EnterBackupScreen({super.key, required this.controller});
  final SignerController controller;

  @override
  State<EnterBackupScreen> createState() => _EnterBackupScreenState();
}

class _EnterBackupScreenState extends State<EnterBackupScreen> {
  static const wordCount = 25;
  final keyNumber = TextEditingController();
  final words = List.generate(wordCount, (_) => TextEditingController());
  final nodes = List.generate(wordCount, (_) => FocusNode());
  String? error;
  bool busy = false;

  @override
  void dispose() {
    keyNumber.dispose();
    for (final c in words) {
      c.dispose();
    }
    for (final n in nodes) {
      n.dispose();
    }
    super.dispose();
  }

  /// Spread whitespace separated words over the fields from the first one on.
  void _fill(String text) {
    final parts = text.split(RegExp(r'\s+')).where((w) => w.isNotEmpty).take(wordCount).toList();
    setState(() {
      for (var i = 0; i < parts.length; i++) {
        words[i].text = parts[i].toUpperCase();
      }
    });
  }

  Future<void> _paste() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (data?.text != null) _fill(data!.text!);
  }

  Future<void> _submit() async {
    final index = int.tryParse(keyNumber.text.trim());
    if (index == null || index < 1) {
      setState(() => error = 'Enter the key number written on the backup (1 or higher).');
      return;
    }
    final entered = [for (final c in words) c.text.trim()];
    final missing = entered.indexWhere((w) => w.isEmpty);
    if (missing >= 0) {
      setState(() => error = 'Word ${missing + 1} is missing.');
      nodes[missing].requestFocus();
      return;
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await widget.controller.submitBackup(index, entered);
      if (mounted) Navigator.pop(context, next);
    } catch (e) {
      if (mounted) setState(() => error = '$e');
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Enter your backup')),
      body: ListView(padding: const EdgeInsets.all(16), children: [
        const Text('Type the key number and the 25 words exactly as written on your backup, in order.'),
        const SizedBox(height: 16),
        TextField(
          controller: keyNumber,
          keyboardType: TextInputType.number,
          inputFormatters: [FilteringTextInputFormatter.digitsOnly],
          decoration: const InputDecoration(labelText: 'Key number', prefixText: '#', border: OutlineInputBorder()),
        ),
        const SizedBox(height: 12),
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(onPressed: _paste, icon: const Icon(Icons.paste), label: const Text('Paste all words')),
        ),
        for (var i = 0; i < wordCount; i++)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: _WordField(
              number: i + 1,
              controller: words[i],
              node: nodes[i],
              onSubmitted: () => i + 1 < wordCount ? nodes[i + 1].requestFocus() : _submit(),
              onPasted: _fill,
            ),
          ),
        if (error != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(error!, style: TextStyle(color: scheme.error)),
          ),
        const SizedBox(height: 8),
        FilledButton(
          onPressed: busy ? null : _submit,
          child: busy
              ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('Check backup'),
        ),
      ]),
    );
  }
}

class _WordField extends StatelessWidget {
  const _WordField({
    required this.number,
    required this.controller,
    required this.node,
    required this.onSubmitted,
    required this.onPasted,
  });
  final int number;
  final TextEditingController controller;
  final FocusNode node;
  final VoidCallback onSubmitted;
  final void Function(String) onPasted;

  @override
  Widget build(BuildContext context) {
    return RawAutocomplete<String>(
      textEditingController: controller,
      focusNode: node,
      optionsBuilder: (value) {
        final prefix = value.text.trim();
        if (prefix.isEmpty || prefix.contains(RegExp(r'\s'))) return const Iterable<String>.empty();
        return backupWordSuggestions(prefix: prefix).take(6);
      },
      onSelected: (_) => onSubmitted(),
      fieldViewBuilder: (context, controller, node, onFieldSubmitted) => TextField(
        controller: controller,
        focusNode: node,
        autocorrect: false,
        enableSuggestions: false,
        textCapitalization: TextCapitalization.characters,
        textInputAction: TextInputAction.next,
        style: const TextStyle(fontFamily: 'monospace', fontWeight: FontWeight.bold),
        decoration: InputDecoration(labelText: 'Word $number', border: const OutlineInputBorder(), isDense: true),
        onChanged: (text) {
          // pasting several words into one field fills the following fields
          if (text.trim().contains(RegExp(r'\s'))) onPasted(text);
        },
        onSubmitted: (_) {
          // a single remaining suggestion is taken on "next"
          final options = backupWordSuggestions(prefix: controller.text);
          if (options.length == 1) controller.text = options.first;
          onSubmitted();
        },
      ),
      optionsViewBuilder: (context, onSelected, options) => Align(
        alignment: Alignment.topLeft,
        child: Material(
          elevation: 4,
          child: SizedBox(
            width: 220,
            child: ListView(
              padding: EdgeInsets.zero,
              shrinkWrap: true,
              children: [
                for (final o in options)
                  ListTile(dense: true, title: Text(o, style: const TextStyle(fontFamily: 'monospace')), onTap: () => onSelected(o)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
