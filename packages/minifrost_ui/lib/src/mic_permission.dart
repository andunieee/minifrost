import 'dart:io' show Platform;

import 'package:permission_handler/permission_handler.dart';

/// Asks for the microphone on Android (listening for data over sound). Other platforms have no
/// runtime prompt, so this is true there.
Future<bool> ensureMicrophone() async {
  if (!Platform.isAndroid) return true;
  return (await Permission.microphone.request()).isGranted;
}
