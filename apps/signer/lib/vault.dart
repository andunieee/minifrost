
import 'package:flutter/services.dart';

class VaultStatus {
  const VaultStatus({required this.exists, required this.secureLockScreen, required this.hardware});
  final bool exists;
  final bool secureLockScreen;

  /// "strongbox", "tee", "software" or "none"
  final String hardware;

  String get hardwareLabel => switch (hardware) {
        'strongbox' => 'StrongBox secure element',
        'tee' => 'Trusted execution environment',
        'software' => 'Software only (weak!)',
        _ => 'Not created yet',
      };
}

class VaultException implements Exception {
  VaultException(this.code, this.message);
  final String code;
  final String message;
  @override
  String toString() => message;
}

/// Encrypted-at-rest storage backed by the Android Keystore (see Vault.kt).
class Vault {
  static const _ch = MethodChannel('minifrost/vault');

  Future<VaultStatus> status() async {
    final m = await _ch.invokeMapMethod<String, dynamic>('status') ?? {};
    return VaultStatus(
      exists: m['exists'] == true,
      secureLockScreen: m['secureLockScreen'] == true,
      hardware: (m['hardware'] as String?) ?? 'none',
    );
  }

  /// Prompts for biometrics / screen lock when needed.
  Future<Uint8List?> load() async {
    try {
      return await _ch.invokeMethod<Uint8List>('load');
    } on PlatformException catch (e) {
      throw VaultException(e.code, e.message ?? e.code);
    }
  }

  /// Completes only once the data is durably written.
  Future<void> save(Uint8List bytes) async {
    try {
      await _ch.invokeMethod<void>('save', bytes);
    } on PlatformException catch (e) {
      throw VaultException(e.code, e.message ?? e.code);
    }
  }

  Future<void> wipe() => _ch.invokeMethod<void>('wipe');
}
