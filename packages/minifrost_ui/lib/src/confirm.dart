import 'package:flutter/material.dart';

/// A yes/no question. Returns true only when [action] was pressed. [danger] draws it in the
/// error colour, for what can't be undone.
Future<bool> confirmDialog(BuildContext context,
        {required String title, required String body, required String action, bool danger = false}) async =>
    await showDialog<bool>(
      context: context,
      builder: (dialog) => AlertDialog(
        icon: danger ? Icon(Icons.warning_amber_rounded, color: Theme.of(dialog).colorScheme.error) : null,
        title: Text(title),
        content: SingleChildScrollView(child: Text(body)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(dialog, false), child: const Text('Cancel')),
          FilledButton(
            style: danger ? _dangerStyle(dialog) : null,
            onPressed: () => Navigator.pop(dialog, true),
            child: Text(action),
          ),
        ],
      ),
    ) ==
    true;

/// The last step before something that can't be undone: [action] only unlocks once [name] is typed
/// in exactly.
Future<bool> confirmByTyping(BuildContext context,
        {required String title, required String body, required String name, required String action}) async =>
    await showDialog<bool>(
      context: context,
      builder: (_) => _TypeToConfirm(title: title, body: body, name: name, action: action),
    ) ==
    true;

ButtonStyle _dangerStyle(BuildContext context) {
  final scheme = Theme.of(context).colorScheme;
  return FilledButton.styleFrom(backgroundColor: scheme.error, foregroundColor: scheme.onError);
}

class _TypeToConfirm extends StatefulWidget {
  const _TypeToConfirm({required this.title, required this.body, required this.name, required this.action});
  final String title;
  final String body;
  final String name;
  final String action;
  @override
  State<_TypeToConfirm> createState() => _TypeToConfirmState();
}

class _TypeToConfirmState extends State<_TypeToConfirm> {
  final typed = TextEditingController();

  @override
  void dispose() {
    typed.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ok = typed.text.trim() == widget.name.trim();
    return AlertDialog(
      icon: Icon(Icons.warning_amber_rounded, color: Theme.of(context).colorScheme.error),
      title: Text(widget.title),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(widget.body),
            const SizedBox(height: 16),
            Text('Type “${widget.name}” to confirm.'),
            TextField(controller: typed, autofocus: true, onChanged: (_) => setState(() {})),
          ]),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
        FilledButton(
          style: _dangerStyle(context),
          onPressed: ok ? () => Navigator.pop(context, true) : null,
          child: Text(widget.action),
        ),
      ],
    );
  }
}
