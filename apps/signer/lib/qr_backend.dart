import 'dart:typed_data';

import 'package:minifrost_ui/minifrost_ui.dart';

import 'src/rust/api.dart' as rust;
import 'src/rust/api/camera.dart' as cam;
import 'src/rust/api/sound.dart' as sound;

/// Hooks the shared scan panel up to this app's flutter_rust_bridge bindings.
class RustQrBackend implements QrScanBackend {
  const RustQrBackend();

  @override
  Future<QrAssembler> newAssembler() async => _Assembler(await rust.newQrScanner());

  @override
  Future<List<String>> decodeLuma({required int width, required int height, required int rowStride, required Uint8List luma}) =>
      rust.decodeQrLuma(width: width, height: height, rowStride: rowStride, luma: luma);

  @override
  Future<List<QrCameraInfo>> listCameras() async =>
      [for (final c in await cam.listCameras()) QrCameraInfo(index: c.index, name: c.name)];

  @override
  Stream<QrCameraEvent> startCamera(int index) => cam.startCamera(index: index).map(
        (e) => switch (e) {
          cam.CameraEvent_Preview(:final field0) => QrCameraPreview(field0),
          cam.CameraEvent_Qr(:final field0) => QrCameraText(field0),
        },
      );

  @override
  void stopCamera() => cam.stopCamera();

  @override
  Stream<String> listenSound() => sound.startSoundListening();

  @override
  void stopListeningSound() => sound.stopSoundListening();
}

/// Plays frames through the speaker with ggwave.
class RustSoundPlayer implements SoundPlayer {
  const RustSoundPlayer();

  @override
  Future<void> play(List<String> frames) => sound.startSoundPlayback(frames: frames);

  @override
  void stop() => sound.stopSoundPlayback();
}

class _Assembler implements QrAssembler {
  _Assembler(this._scanner);
  final rust.QrScanner _scanner;

  @override
  Future<QrScanProgress> push(String text) async {
    final p = await _scanner.push(text: text);
    return QrScanProgress(received: p.received, total: p.total, payload: p.payload, problem: p.problem);
  }
}
