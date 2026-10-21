import 'package:flutter/material.dart';

import 'src/rust/frb_generated.dart';
import 'app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const SignerRoot());
}
