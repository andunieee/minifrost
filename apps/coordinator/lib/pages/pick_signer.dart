import 'package:flutter/material.dart';

import '../src/rust/engine.dart';

/// Ask which enrolled signer a backup action is for. Returns null if cancelled.
Future<SignerInfo?> pickSigner(
  BuildContext context, {
  required List<SignerInfo> signers,
  required String title,
  String? hint,
}) {
  return showDialog<SignerInfo>(
    context: context,
    builder: (_) => AlertDialog(
      title: Text(title),
      content: SizedBox(
        width: 420,
        child: signers.isEmpty
            ? const Text('No signers to choose from. Add the signer phone on the Signers page first.')
            : Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
                if (hint != null) Padding(padding: const EdgeInsets.only(bottom: 8), child: Text(hint)),
                for (final s in signers)
                  ListTile(
                    leading: const Icon(Icons.phone_android),
                    title: Text(s.name),
                    onTap: () => Navigator.pop(context, s),
                  ),
              ]),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel'))],
    ),
  );
}

/// Show a snackbar with an error.
void toast(BuildContext context, Object message) {
  if (!context.mounted) return;
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$message')));
}
