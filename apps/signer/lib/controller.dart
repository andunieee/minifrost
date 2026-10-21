
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

  Future<void> refresh() async {
    keys = await signer.keys();
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
}
