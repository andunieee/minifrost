import 'dart:io' show File, Platform;

import 'package:flutter/foundation.dart' show Uint8List;
import 'package:path_provider/path_provider.dart';

class VaultStatus {
  const VaultStatus({required this.exists});
  final bool exists;
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

Vault createVault() => FileVault();

/// A plain, unencrypted file in the app-support directory, written atomically (temp file +
/// rename). On Android that is the app-private directory, excluded from backups.
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
    return VaultStatus(exists: await f.exists());
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
