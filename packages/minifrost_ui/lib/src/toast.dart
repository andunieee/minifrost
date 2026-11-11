import 'package:flutter/material.dart';

/// A green "it worked" snackbar, replacing any toast still on screen. [color] overrides the
/// green, e.g. with the colour of the signer the toast is about.
void showSuccessToast(BuildContext context, String message, {Color? color}) {
  _showToast(context, message, color ?? Colors.green.shade800, Icons.check_circle);
}

/// A neutral "nothing new" snackbar, replacing any toast still on screen so repeats never queue
/// up behind each other and delay the next real result.
void showInfoToast(BuildContext context, String message, {Color? color}) {
  _showToast(context, message, color ?? Colors.blueGrey.shade700, Icons.info_outline,
      duration: const Duration(seconds: 2));
}

void _showToast(BuildContext context, String message, Color background, IconData icon,
    {Duration duration = const Duration(seconds: 4)}) {
  final messenger = ScaffoldMessenger.maybeOf(context);
  if (messenger == null) return;
  // white text needs a dark enough background: signer colours are light in dark mode
  final foreground = ThemeData.estimateBrightnessForColor(background) == Brightness.dark ? Colors.white : Colors.black;
  messenger
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(
      backgroundColor: background,
      duration: duration,
      content: Row(children: [
        Icon(icon, color: foreground),
        const SizedBox(width: 12),
        Expanded(child: Text(message, style: TextStyle(color: foreground))),
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

/// Show a snackbar with an error.
void toast(BuildContext context, Object message) {
  if (!context.mounted) return;
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$message')));
}
