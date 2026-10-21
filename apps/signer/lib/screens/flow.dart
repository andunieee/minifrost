import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../controller.dart';
import '../src/rust/engine.dart';
import 'prompt.dart';
import 'reply.dart';
import 'scan.dart';

/// Scan a coordinator code and walk the user through whatever it asks.
Future<void> runScanFlow(BuildContext context, SignerController c) async {
  final payload = await Navigator.push<Uint8List>(context, MaterialPageRoute(builder: (_) => const ScanScreen()));
  if (payload == null || !context.mounted) return;
  await _handle(context, c, () => c.receive(payload));
}

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
    case Step_Done(:final message):
      await showDialog<void>(
        context: context,
        builder: (_) => AlertDialog(
          title: const Text('Done'),
          content: Text(message),
          actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('OK'))],
        ),
      );
    case Step_Reply(:final frames, :final what):
      await nav.push(MaterialPageRoute(builder: (_) => ReplyScreen(frames: frames, title: 'Show to coordinator', what: what)));
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
