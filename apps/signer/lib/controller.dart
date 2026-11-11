
import 'package:flutter/foundation.dart';

import 'keystore.dart';
import 'src/rust/api.dart';
import 'src/rust/engine.dart';
import 'vault.dart';

/// Glue between the Rust state machine and secure storage. The one rule it enforces: whenever
/// Rust says its state changed, the new snapshot is durably stored BEFORE the caller gets the
/// result (and therefore before any QR code is shown). After a partial signature the snapshot
/// contains the burned nonce; showing the signature first would risk nonce reuse after a crash.
class SignerController extends ChangeNotifier {
  SignerController(this.signer, this.vault, this.deviceId);

  final SignerApp signer;
  final Vault vault;
  final String deviceId;
  final ShareKeystore keystore = ShareKeystore();
  List<KeyInfo> keys = [];
  List<PendingBackup> pendingBackups = [];
  bool newSharesOnKeystore = false;

  Future<void> refresh() async {
    keys = await signer.keys();
    pendingBackups = await signer.pendingBackups();
    newSharesOnKeystore = await signer.newSharesOnKeystore();
    notifyListeners();
  }

  Future<StepResult> _persist(StepResult r) async {
    final snap = r.snapshot;
    if (snap != null) await vault.save(snap);
    await refresh();
    final key = r.newShare;
    if (key != null && newSharesOnKeystore && keystore.supported) return _moveNewShare(r, key);
    return r;
  }

  /// "Store new shares on the device key store": the share was just stored on disk, move it now.
  /// When that fails the share simply stays on disk, and the step's message says so.
  Future<StepResult> _moveNewShare(StepResult r, String keyId) async {
    String? note;
    try {
      await moveToKeystore(keyId);
      note = 'The share is on the device key store.';
    } catch (e) {
      note = 'The share stayed on disk: $e You can move it to the device key store from its menu on the home screen.';
    }
    final step = r.step;
    if (step is! Step_Done) return r;
    return StepResult(step: Step.done(message: '${step.message}\n\n$note'), snapshot: r.snapshot, newShare: r.newShare);
  }

  Future<StepResult> receive(Uint8List payload) async {
    var r = await _persist(await signer.receive(payload: payload));
    final step = r.step;
    if (step is Step_UnlockShare) {
      // the message needs a share on the key store: open it (screen lock), then read it again
      await openShare(step.keyId);
      r = await _persist(await signer.receive(payload: payload));
    }
    return r;
  }

  // ---- device key store

  Future<void> setNewSharesOnKeystore(bool enabled) async {
    await vault.save(await signer.setNewSharesOnKeystore(enabled: enabled));
    await refresh();
  }

  Future<void> moveToKeystore(String keyId) async {
    final sealed = await keystore.seal(await signer.shareForKeystore(keyId: keyId));
    await vault.save(await signer.movedToKeystore(keyId: keyId, sealed: sealed));
    await refresh();
  }

  /// Opens a share on the key store for the rest of this session.
  Future<void> openShare(String keyId) async {
    final share = await keystore.open(await signer.keystoreShare(keyId: keyId));
    await signer.openKeystoreShare(keyId: keyId, share: share);
  }

  Future<void> moveToDisk(String keyId) async {
    await openShare(keyId);
    await vault.save(await signer.moveToDisk(keyId: keyId));
    await refresh();
  }
  // ---- deleting

  /// Delete this signer's share of a key for good. A share on the key store goes with its sealed
  /// copy; nothing needs to be opened for that.
  Future<void> deleteKey(String keyId) async {
    await vault.save(await signer.deleteKey(keyId: keyId));
    await refresh();
  }

  Future<void> deletePendingBackup(String id) async {
    await vault.save(await signer.deletePendingBackup(id: id));
    await refresh();
  }

  Future<StepResult> approve() async => _persist(await signer.approve());
  Future<void> reject() => signer.reject();

  /// The user typed a physical backup (key number + 25 words). Throws a readable message when the
  /// words don't form a valid backup, so they can be corrected. Otherwise the backup is saved and
  /// the step is its restore code.
  Future<StepResult> submitBackup(int shareIndex, List<String> words) async =>
      _persist(await signer.submitBackup(shareIndex: shareIndex, words: words));

  /// The restore code of a key this signer holds. A share on the key store is opened first (screen
  /// lock), since the code carries the share's public image.
  Future<Step> restoreCode(String keyId) async {
    final step = await signer.restoreCode(keyId: keyId);
    if (step is! Step_UnlockShare) return step;
    await openShare(keyId);
    return signer.restoreCode(keyId: keyId);
  }

  Future<QuizQuestion> quizQuestion() => signer.quizQuestion();

  Future<QuizAnswer> quizAnswer(int choice) async {
    final answer = await signer.quizAnswer(choice: choice);
    final done = answer.done;
    if (done != null) await _persist(done);
    return answer;
  }
}
