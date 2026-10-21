import 'dart:io' show File, Platform;

import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

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

abstract class Vault {
  Future<VaultStatus> status();
  Future<Uint8List?> load();
  Future<void> save(Uint8List bytes);
  Future<void> wipe();
}

Vault createVault() => Platform.isAndroid ? AndroidVault() : FileVault();

/// Encrypted-at-rest storage backed by the Android Keystore (see Vault.kt).
class AndroidVault implements Vault {
  static const _ch = MethodChannel('minifrost/vault');

  @override
  Future<VaultStatus> status() async {
    final m = await _ch.invokeMapMethod<String, dynamic>('status') ?? {};
    return VaultStatus(
      exists: m['exists'] == true,
      secureLockScreen: m['secureLockScreen'] == true,
      hardware: (m['hardware'] as String?) ?? 'none',
    );
  }

  /// Prompts for biometrics / screen lock when needed.
  @override
  Future<Uint8List?> load() async {
    try {
      return await _ch.invokeMethod<Uint8List>('load');
    } on PlatformException catch (e) {
      throw VaultException(e.code, e.message ?? e.code);
    }
  }

  /// Completes only once the data is durably written.
  @override
  Future<void> save(Uint8List bytes) async {
    try {
      await _ch.invokeMethod<void>('save', bytes);
    } on PlatformException catch (e) {
      throw VaultException(e.code, e.message ?? e.code);
    }
  }

  @override
  Future<void> wipe() => _ch.invokeMethod<void>('wipe');
}

/// Desktop backing: a plain file in the app-support directory, written atomically (temp file +
/// rename). This is for dev/testing only — the key share sits in the filesystem with no hardware
/// or OS-keyring protection, which `VaultStatus.hardwareLabel` makes sure to say out loud.
class FileVault implements Vault {
  static const _name = 'minifrost.vault';
  File? _file;

  Future<File> _open() async {
    if (_file != null) return _file!;
    final dir = await getApplicationSupportDirectory();
    return _file = File('${dir.path}${Platform.pathSeparator}$_name');
  }

  @override
  Future<VaultStatus> status() async {
    final f = await _open();
    return VaultStatus(
      exists: await f.exists(),
      secureLockScreen: true, // desktop: nothing checks a screen lock for us
      hardware: 'software',
    );
  }

  @override
  Future<Uint8List?> load() async {
    final f = await _open();
    if (!await f.exists()) return null;
    return await f.readAsBytes();
  }

  @override
  Future<void> save(Uint8List bytes) async {
    final f = await _open();
    await f.parent.create(recursive: true);
    final tmp = File('${f.path}.tmp');
    await tmp.writeAsBytes(bytes, flush: true);
    await tmp.rename(f.path);
  }

  @override
  Future<void> wipe() async {
    final f = await _open();
    if (await f.exists()) await f.delete();
  }
}
