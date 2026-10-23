import 'dart:typed_data';

/// The Rust-backed pieces [ScanPanel] needs. Each app generates its own flutter_rust_bridge
/// bindings, so this interface is what keeps the widget independent of them: an app wraps its
/// generated `api.dart` / `api/camera.dart` in one small adapter.
abstract class QrScanBackend {
  /// A fresh frame assembler. One per scanning session.
  Future<QrAssembler> newAssembler();

  /// Decode QR texts out of one camera frame's luma (Y) plane (phone cameras).
  Future<List<String>> decodeLuma({required int width, required int height, required int rowStride, required Uint8List luma});

  /// Desktop webcams. May throw where webcam scanning isn't available.
  Future<List<QrCameraInfo>> listCameras();

  /// Starts [index] (stopping any previous stream); the stream carries preview JPEGs and QR texts.
  Stream<QrCameraEvent> startCamera(int index);
  void stopCamera();
}

abstract class QrAssembler {
  Future<QrScanProgress> push(String text);
}

class QrScanProgress {
  const QrScanProgress({required this.received, required this.total, this.payload, this.problem});
  final int received;
  final int total;

  /// Set once every frame of a code has arrived.
  final Uint8List? payload;

  /// Set when the scanned QR is not ours or is damaged.
  final String? problem;
}

class QrCameraInfo {
  const QrCameraInfo({required this.index, required this.name});
  final int index;
  final String name;
}

sealed class QrCameraEvent {
  const QrCameraEvent();
}

class QrCameraPreview extends QrCameraEvent {
  const QrCameraPreview(this.jpeg);
  final Uint8List jpeg;
}

class QrCameraText extends QrCameraEvent {
  const QrCameraText(this.text);
  final String text;
}
