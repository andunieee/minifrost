import 'package:flutter/material.dart';

import '../scan_panel.dart';
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
                  leading: const Icon(Icons.phone_android),
                  title: Text(s.name),
                  subtitle: Text('${s.deviceId.substring(0, 24)}…', style: const TextStyle(fontFamily: 'monospace')),
                  trailing: IconButton(
                    icon: const Icon(Icons.delete_outline),
                    tooltip: 'Forget',
                    onPressed: () async {
                      try {
                        await widget.app.forgetSigner(deviceId: s.deviceId);
                        _reload();
                      } catch (e) {
                        if (context.mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
                      }
                    },
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
      content: SizedBox(
        width: 480,
        child: added != null
            ? Text('Added “${added!.name}”.')
            : ScanPanel(
                hint: 'Show the signer’s enrolment code to the camera',
                onPayload: (bytes) async {
                  try {
                    final s = await widget.app.enroll(payload: bytes);
                    widget.onAdded();
                    if (mounted) setState(() => added = s);
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
