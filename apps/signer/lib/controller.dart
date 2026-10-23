
import 'package:flutter/foundation.dart';

import 'src/rust/api.dart';
import 'src/rust/engine.dart';
import 'vault.dart';

/// Glue between the Rust state machine and secure storage. The one rule it enforces: whenever
/// Rust says its state changed, the new snapshot is durably stored BEFORE the caller gets the
/// result (and therefore before any QR code is shown). After a partial signature the snapshot
/// contains the burned nonce; showing the signature first would risk nonce reuse after a crash.
class SignerController extends ChangeNotifier {
  SignerController(this.signer, this.vault);

  final SignerApp signer;
  final Vault vault;
  List<KeyInfo> keys = [];
  List<PendingBackup> pendingBackups = [];

  Future<void> refresh() async {
    keys = await signer.keys();
    pendingBackups = await signer.pendingBackups();
    notifyListeners();
  }

  Future<StepResult> _persist(StepResult r) async {
    final snap = r.snapshot;
    if (snap != null) await vault.save(snap);
    await refresh();
    return r;
  }

  Future<StepResult> receive(Uint8List payload) async => _persist(await signer.receive(payload: payload));
  Future<StepResult> approve() async => _persist(await signer.approve());
  Future<void> reject() => signer.reject();

  /// The user typed a physical backup (key number + 25 words). Throws a readable message when the
  /// words don't form a valid backup; the signer then keeps waiting for a corrected attempt.
  Future<StepResult> submitBackup(int shareIndex, List<String> words) async =>
      _persist(await signer.submitBackup(shareIndex: shareIndex, words: words));

  Future<QuizQuestion> quizQuestion() => signer.quizQuestion();

  Future<QuizAnswer> quizAnswer(int choice) async {
    final answer = await signer.quizAnswer(choice: choice);
    final done = answer.done;
    if (done != null) await _persist(done);
    return answer;
  }
}
