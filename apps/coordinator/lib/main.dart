import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import 'app.dart';
import 'src/rust/api.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final dir = await getApplicationSupportDirectory();
  final app = await openCoordinator(dir: dir.path);
  runApp(CoordinatorRoot(app: app));
}
