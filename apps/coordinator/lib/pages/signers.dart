import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../qr_backend.dart';
import '../src/rust/api.dart';
import '../src/rust/engine.dart';

class SignersPage extends StatefulWidget {
  const SignersPage({super.key, required this.app});
  final CoordinatorApp app;
  @override
  State<SignersPage> createState() => _SignersPageState();
}

class _SignersPageState extends State<SignersPage> {
  late Future<List<SignerInfo>> signers = widget.app.signers();

  void _reload() => setState(() {
    signers = widget.app.signers();
  });

  Future<void> _add() async {
    await showDialog<void>(context: context, builder: (_) => _EnrollDialog(app: widget.app, onAdded: _reload));
    _reload();
  }

  /// Forget a signer, and with it the keys this coordinator knows it holds. The signer keeps them.
  Future<void> _forget(SignerInfo s) async {
    final held = [
      for (final k in await widget.app.keys())
        for (final h in k.signers)
          if (h.deviceId == s.deviceId) (k, h.shareIndex),
    ];
    if (!mounted) return;
    final effect = held.isEmpty
        ? '${s.name} holds no key of a wallet here.'
        : '${s.name} stops counting for:\n'
            '${held.map((h) => '• “${h.$1.name}”, key #${h.$2}${_lastHolder(h.$1, h.$2) ? ' (no other signer holds it)' : ''}').join('\n')}';
    final short = held.any((h) => _lastHolder(h.$1, h.$2) && h.$1.nKeys - 1 < h.$1.threshold);
    final ok = await confirmDialog(
      context,
      title: 'Forget ${s.name}?',
      body: '$effect\n\n'
          '${short ? 'A wallet is then left with fewer keys than it needs to sign, until a signer with that key joins again.\n\n' : ''}'
          'Nothing changes on the signer: it keeps its keys. To add it back, enrol it again, and show the '
          'restore code of each key (tap the key on the signer) on the Restore page.',
      action: 'Forget',
      danger: held.isNotEmpty,
    );
    if (!ok) return;
    try {
      await widget.app.forgetSigner(deviceId: s.deviceId);
    } catch (e) {
      if (mounted) toast(context, e);
    }
    _reload();
  }

  static bool _lastHolder(KeyView k, int shareIndex) => k.signers.where((h) => h.shareIndex == shareIndex).length == 1;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Signers')),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _add,
        icon: const Icon(Icons.qr_code_scanner),
        label: const Text('Add signer'),
      ),
      body: FutureBuilder(
        future: signers,
        builder: (context, snap) {
          final list = snap.data;
          if (list == null) return const Center(child: CircularProgressIndicator());
          if (list.isEmpty) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.all(24),
                child: Text('No signers yet. On each signer phone choose “Show enrolment code”, then add it here.',
                    textAlign: TextAlign.center),
              ),
            );
          }
          return ListView(padding: const EdgeInsets.all(16), children: [
            for (final s in list)
              Card(
                child: ListTile(
                  leading: Icon(Icons.phone_android, color: signerColor(s.deviceId, Theme.of(context).brightness)),
                  title: SignerName(deviceId: s.deviceId, name: s.name),
                  subtitle: Text('${s.deviceId.substring(0, 24)}…', style: const TextStyle(fontFamily: 'monospace')),
                  trailing: IconButton(
                    icon: const Icon(Icons.delete_outline),
                    tooltip: 'Forget',
                    onPressed: () => _forget(s),
                  ),
                ),
              ),
          ]);
        },
      ),
    );
  }
}

class _EnrollDialog extends StatefulWidget {
  const _EnrollDialog({required this.app, required this.onAdded});
  final CoordinatorApp app;
  final VoidCallback onAdded;
  @override
  State<_EnrollDialog> createState() => _EnrollDialogState();
}

class _EnrollDialogState extends State<_EnrollDialog> {
  SignerInfo? added;
  String? error;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add signer'),
      scrollable: true,
      content: SizedBox(
        width: 480,
        child: added != null
            ? Text('Added “${added!.name}”.')
            : ScanPanel(
                backend: const RustQrBackend(),
                hint: 'Show the signer’s enrolment code to the camera',
                expecting: 'signer enrolment code',
                onPayload: (bytes) async {
                  try {
                    final s = await widget.app.enroll(payload: bytes);
                    widget.onAdded();
                    if (!mounted) return;
                    showSuccessToast(this.context, 'Read enrolment code: added “${s.name}”.');
                    setState(() => added = s);
                  } catch (e) {
                    if (mounted) setState(() => error = '$e');
                  }
                },
              ),
      ),
      actions: [
        if (error != null) Text(error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
        TextButton(onPressed: () => Navigator.pop(context), child: Text(added != null ? 'Done' : 'Cancel')),
      ],
    );
  }
}
