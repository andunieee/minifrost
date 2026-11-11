import 'package:flutter/material.dart';
import 'package:minifrost_ui/minifrost_ui.dart';

import '../controller.dart';
import '../src/rust/engine.dart';
import 'flow.dart';
import 'reply.dart';

class Home extends StatelessWidget {
  const Home({super.key, required this.controller});
  final SignerController controller;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) => Scaffold(
        appBar: AppBar(title: const Text('Minifrost Signer'), actions: [
          if (controller.keystore.supported)
            PopupMenuButton<void>(
              itemBuilder: (_) => [
                CheckedPopupMenuItem(
                  checked: controller.newSharesOnKeystore,
                  onTap: () => _toggleNewSharesOnKeystore(context, controller),
                  child: const Text('Store new shares on the device key store'),
                ),
              ],
            ),
        ]),
        body: ListView(padding: const EdgeInsets.all(16), children: [
          FutureBuilder(
            future: Future.wait([controller.signer.name(), controller.signer.deviceId()]),
            builder: (context, snap) {
              final d = snap.data;
              final color = d == null ? null : signerColor(d[1], Theme.of(context).brightness);
              return Card(
                shape: color == null
                    ? null
                    : RoundedRectangleBorder(borderRadius: BorderRadius.circular(12), side: BorderSide(color: color, width: 2)),
                child: ListTile(
                  leading: Icon(Icons.shield_outlined, color: color),
                  title: d == null ? const Text('…') : SignerName(deviceId: d[1], name: d[0], style: Theme.of(context).textTheme.titleMedium),
                  subtitle: Text(d == null ? '' : 'id ${d[1].substring(0, 16)}…',
                      style: const TextStyle(fontFamily: 'monospace')),
                ),
              );
            },
          ),
          const SizedBox(height: 8),
          Text('Keys held by this signer', style: Theme.of(context).textTheme.titleMedium),
          if (controller.keys.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16),
              child: Text('None yet. Show your enrolment code to a coordinator, then scan its key generation codes, or restore a key from its backup.'),
            ),
          for (final k in controller.keys) _KeyTile(k, controller),
          if (controller.pendingBackups.isNotEmpty) ...[
            const SizedBox(height: 16),
            Text('Restoring', style: Theme.of(context).textTheme.titleMedium),
            for (final b in controller.pendingBackups)
              Card(
                child: ListTile(
                  leading: const Icon(Icons.history),
                  title: Text('Backup #${b.shareIndex}${b.keyName == null ? '' : ' of “${b.keyName}”'}'),
                  subtitle: const Text('Saved on this signer. It becomes a normal key once the coordinator has rebuilt the wallet. Tap to show its restore code.'),
                  isThreeLine: true,
                  onTap: () => runRestoreCode(context, controller, () async => b.restoreCode),
                ),
              ),
          ],
          const SizedBox(height: 16),
          OutlinedButton.icon(
            icon: const Icon(Icons.edit_note),
            label: const Text('Restore from a backup'),
            onPressed: () => runEnterBackup(context, controller),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            icon: const Icon(Icons.qr_code),
            label: const Text('Show enrolment code'),
            onPressed: () async {
              final frames = await controller.signer.helloFrames();
              if (!context.mounted) return;
              Navigator.push(
                context,
                MaterialPageRoute(
                  builder: (_) => ReplyScreen(
                    frames: frames,
                    title: 'Enrolment code',
                    what: 'Let the coordinator scan this to add you as a signer.',
                    deviceId: controller.deviceId,
                  ),
                ),
              );
            },
          ),
        ]),
        floatingActionButton: FloatingActionButton.extended(
          icon: const Icon(Icons.qr_code_scanner),
          label: const Text('Scan coordinator'),
          onPressed: () => runScanFlow(context, controller),
        ),
      ),
    );
  }
}

enum _KeyAction { restore, move }

class _KeyTile extends StatelessWidget {
  const _KeyTile(this.k, this.controller);
  final KeyInfo k;
  final SignerController controller;
  @override
  Widget build(BuildContext context) => Card(
        child: ListTile(
          leading: const Icon(Icons.vpn_key_outlined),
          title: Text(k.name),
          subtitle: Text('${k.threshold}-of-? · ${networkLabel(k.network)} · ${k.onKeystore ? 'on keystore' : 'on disk'}\n'
              '${k.keyId.substring(0, 16)}…'),
          isThreeLine: true,
          onTap: () => _keyMenu(context),
        ),
      );

  Future<void> _keyMenu(BuildContext context) async {
    final action = await showModalBottomSheet<_KeyAction>(
      context: context,
      builder: (sheet) => SafeArea(
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          ListTile(title: Text(k.name), subtitle: Text(k.onKeystore ? 'On keystore' : 'On disk')),
          const Divider(height: 1),
          ListTile(
            leading: const Icon(Icons.qr_code),
            title: const Text('Restore wallet on coordinator'),
            onTap: () => Navigator.pop(sheet, _KeyAction.restore),
          ),
          if (controller.keystore.supported)
            ListTile(
              leading: Icon(k.onKeystore ? Icons.save_outlined : Icons.lock_outline),
              title: Text(k.onKeystore ? 'Move this share back to disk' : 'Move this share to the device key store'),
              onTap: () => Navigator.pop(sheet, _KeyAction.move),
            ),
        ]),
      ),
    );
    if (action == null || !context.mounted) return;
    if (action == _KeyAction.restore) {
      await runRestoreCode(context, controller, () => controller.restoreCode(k.keyId));
      return;
    }
    if (k.onKeystore) {
      final ok = await _confirm(
        context,
        title: 'Move this share back to disk?',
        body: 'Confirm your screen lock to take the share out of the device key store. '
            'It is then stored in this app\'s file, like before.',
        action: 'Move to disk',
      );
      if (ok && context.mounted) await _run(context, () => controller.moveToDisk(k.keyId), 'The share is on disk.');
    } else {
      if (!await _screenLockReady(context, controller) || !context.mounted) return;
      final ok = await _confirm(
        context,
        title: 'Move this share to the device key store?',
        body: 'Using this share (signing, showing or checking its backup) will then need this phone\'s '
            'screen lock: PIN, pattern or password.\n\n$_screenLockRisk\n\n'
            'Make sure this share\'s backup is written down and checked before you continue.',
        action: 'Move',
      );
      if (ok && context.mounted) await _run(context, () => controller.moveToKeystore(k.keyId), 'The share is on the device key store.');
    }
  }
}

const _screenLockRisk = 'If you ever remove the screen lock, Android permanently deletes the device key store key, and '
    'every share on the key store can no longer be used. You would have to restore each one from its backup words.';

Future<void> _toggleNewSharesOnKeystore(BuildContext context, SignerController c) async {
  if (c.newSharesOnKeystore) {
    await _run(context, () => c.setNewSharesOnKeystore(false), 'New shares will be stored on disk.');
    return;
  }
  if (!await _screenLockReady(context, c) || !context.mounted) return;
  final ok = await _confirm(
    context,
    title: 'Store new shares on the device key store?',
    body: 'Every share this signer creates from now on goes to the device key store as soon as it is stored. '
        'Using those shares will need this phone\'s screen lock: PIN, pattern or password. '
        'Shares already on this signer stay where they are.\n\n$_screenLockRisk\n\n'
        'Write down and check the backup of each new share.',
    action: 'Turn on',
  );
  if (ok && context.mounted) await _run(context, () => c.setNewSharesOnKeystore(true), 'New shares will be stored on the device key store.');
}

/// The key store needs a screen lock; says so when there is none.
Future<bool> _screenLockReady(BuildContext context, SignerController c) async {
  if (await c.keystore.screenLockSet()) return true;
  if (!context.mounted) return false;
  await showDialog<void>(
    context: context,
    builder: (dialog) => AlertDialog(
      title: const Text('No screen lock'),
      content: const Text('The device key store needs a screen lock. Set up a PIN, pattern or password in the phone\'s settings first.'),
      actions: [TextButton(onPressed: () => Navigator.pop(dialog), child: const Text('OK'))],
    ),
  );
  return false;
}

Future<bool> _confirm(BuildContext context, {required String title, required String body, required String action}) async =>
    await showDialog<bool>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: Text(title),
        content: SingleChildScrollView(child: Text(body)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(dialog, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(dialog, true), child: Text(action)),
        ],
      ),
    ) ==
    true;

Future<void> _run(BuildContext context, Future<void> Function() action, String done) async {
  final messenger = ScaffoldMessenger.of(context);
  try {
    await action();
    messenger.showSnackBar(SnackBar(content: Text(done)));
  } catch (e) {
    messenger.showSnackBar(SnackBar(content: Text('$e')));
  }
}
