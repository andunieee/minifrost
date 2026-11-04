import 'dart:typed_data';

import 'package:flutter/material.dart' hide Step;
import 'package:minifrost_ui/minifrost_ui.dart';

import '../controller.dart';
import '../src/rust/engine.dart';
import 'enter_backup.dart';
import 'prompt.dart';
import 'quiz.dart';
import 'reply.dart';
import 'scan.dart';

/// Scan a coordinator code and walk the user through whatever it asks.
Future<void> runScanFlow(BuildContext context, SignerController c) async {
  final payload = await Navigator.push<Uint8List>(context, MaterialPageRoute(builder: (_) => const ScanScreen()));
  if (payload == null || !context.mounted) return;
  await _receive(context, c, payload);
}

Future<void> _receive(BuildContext context, SignerController c, Uint8List payload) => _handle(context, c, () async {
      final r = await c.receive(payload);
      final kind = _readKind(r.step);
      if (kind != null && context.mounted) showSuccessToast(context, 'Read from coordinator: $kind');
      return r;
    });

/// What the coordinator code that was just read asked for; null when it needs nothing.
String? _readKind(Step step) => switch (step) {
      Step_Confirm(:final field0) => switch (field0) {
          Prompt_JoinKeygen() => 'key generation invitation',
          Prompt_VerifyKeygen() => 'key generation check code',
          Prompt_ReviewTransaction() => 'transaction to review',
          Prompt_SignTransaction() => 'transaction to sign',
          Prompt_DisplayBackup() => 'backup request',
        },
      Step_EnterBackup() => 'request to enter a backup',
      Step_CheckBackup() => 'request to check a backup',
      Step_Reply() || Step_Done() => 'coordinator message',
      Step_Ignored() || Step_UnlockShare() => null,
    };

Future<void> _handle(BuildContext context, SignerController c, Future<StepResult> Function() action) async {
  final nav = Navigator.of(context);
  final messenger = ScaffoldMessenger.of(context);
  StepResult r;
  try {
    r = await action();
  } catch (e) {
    if (context.mounted) await _error(context, e);
    return;
  }
  if (!context.mounted) return;
  final step = r.step;
  switch (step) {
    case Step_Ignored(:final reason):
      messenger.showSnackBar(SnackBar(content: Text(reason)));
    case Step_UnlockShare(:final keyName):
      // the controller opens the share and reads the code again; this is only left if that failed
      messenger.showSnackBar(SnackBar(content: Text('The share of “$keyName” could not be opened.')));
    case Step_Done(:final message):
      await showDialog<void>(
        context: context,
        builder: (_) => AlertDialog(
          title: const Text('Done'),
          content: Text(message),
          actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('OK'))],
        ),
      );
    case Step_EnterBackup():
      // The screen submits the words itself; it pops with the signer's next step, or null if the
      // user gave up.
      final next = await nav.push<StepResult>(MaterialPageRoute(builder: (_) => EnterBackupScreen(controller: c)));
      if (next == null) {
        await c.reject();
        return;
      }
      if (!context.mounted) return;
      await _handle(context, c, () async => next);
    case Step_CheckBackup(:final keyName):
      final next = await nav.push<StepResult>(MaterialPageRoute(builder: (_) => QuizScreen(controller: c, keyName: keyName)));
      if (next == null) {
        await c.reject();
        return;
      }
      if (!context.mounted) return;
      await _handle(context, c, () async => next);
    case Step_Reply(:final frames, :final what, :final next):
      // pops with the coordinator's following code when the user scanned it from there
      final payload = await nav.push<Uint8List>(MaterialPageRoute(
          builder: (_) => ReplyScreen(frames: frames, title: 'Show to coordinator', what: what, deviceId: c.deviceId, next: next)));
      if (payload == null || !context.mounted) return;
      await _receive(context, c, payload);
    case Step_Confirm(:final field0):
      await nav.push(MaterialPageRoute(
        builder: (promptContext) => PromptScreen(
          prompt: field0,
          onReject: () async {
            await c.reject();
            if (promptContext.mounted) Navigator.pop(promptContext);
          },
          onApprove: () async {
            StepResult next;
            try {
              next = await c.approve();
            } catch (e) {
              if (promptContext.mounted) await _error(promptContext, e);
              return;
            }
            if (!promptContext.mounted) return;
            // replace the prompt with whatever comes next
            Navigator.pop(promptContext);
            if (!context.mounted) return;
            await _handle(context, c, () async => next);
          },
        ),
      ));
  }
}

Future<void> _error(BuildContext context, Object e) => showDialog<void>(
      context: context,
      builder: (_) => AlertDialog(
        icon: const Icon(Icons.error_outline),
        title: const Text('Can’t continue'),
        content: Text('$e'),
        actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('OK'))],
      ),
    );
