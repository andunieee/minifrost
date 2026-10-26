import 'package:flutter/material.dart';

/// A green "it worked" snackbar, replacing any toast still on screen.
void showSuccessToast(BuildContext context, String message) {
  final messenger = ScaffoldMessenger.maybeOf(context);
  if (messenger == null) return;
  messenger
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(
      backgroundColor: Colors.green.shade800,
      content: Row(children: [
        const Icon(Icons.check_circle, color: Colors.white),
        const SizedBox(width: 12),
        Expanded(child: Text(message, style: const TextStyle(color: Colors.white))),
      ]),
    ));
}

/// A small pill naming the kind of message a QR code carries or is expected to carry.
class QrKindLabel extends StatelessWidget {
  const QrKindLabel(this.text, {super.key, this.icon = Icons.qr_code_2});
  final String text;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Center(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(color: scheme.secondaryContainer, borderRadius: BorderRadius.circular(16)),
        child: Row(mainAxisSize: MainAxisSize.min, children: [
          Icon(icon, size: 18, color: scheme.onSecondaryContainer),
          const SizedBox(width: 6),
          Flexible(child: Text(text, style: TextStyle(color: scheme.onSecondaryContainer, fontWeight: FontWeight.w600))),
        ]),
      ),
    );
  }
}
