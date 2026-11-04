import 'dart:io' show Platform;

import 'package:flutter/services.dart';

class KeystoreException implements Exception {
  KeystoreException(this.code, this.message);
  final String code;
  final String message;
  @override
  String toString() => message;
}

/// The Android key store, through `ShareKeystore.kt`. Sealing and opening ask for the screen lock
/// when it wasn't confirmed in the last 30 seconds. Not available on the Linux desktop build.
class ShareKeystore {
  static const _channel = MethodChannel('minifrost/keystore');

  bool get supported => Platform.isAndroid;

  Future<bool> screenLockSet() async => supported && await _channel.invokeMethod<bool>('screenLockSet') == true;

  Future<Uint8List> seal(Uint8List share) => _call('seal', share);
  Future<Uint8List> open(Uint8List sealed) => _call('open', sealed);

  Future<Uint8List> _call(String method, Uint8List bytes) async {
    try {
      return (await _channel.invokeMethod<Uint8List>(method, bytes))!;
    } on PlatformException catch (e) {
      throw KeystoreException(e.code, switch (e.code) {
        'key_gone' => 'The device key store no longer has the key this share was stored with. '
            'Android deletes it when the screen lock is removed. Restore this share from its backup words.',
        _ => e.message ?? e.code,
      });
    }
  }
}
