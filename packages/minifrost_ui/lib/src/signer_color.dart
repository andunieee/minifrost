import 'package:flutter/material.dart';

/// A colour that belongs to one signer, derived from its device id so the signer and the
/// coordinator agree on it without exchanging anything.
Color signerColor(String deviceId, Brightness brightness) {
  // the id is hex of a public key: a few leading digits are uniformly distributed
  final seed = int.tryParse(deviceId.substring(0, deviceId.length < 8 ? deviceId.length : 8), radix: 16) ?? 0;
  final hue = (seed % 360).toDouble();
  final dark = brightness == Brightness.dark;
  return HSLColor.fromAHSL(1, hue, 0.65, dark ? 0.68 : 0.38).toColor();
}

/// A dark shade of [color] that still reads as that hue but keeps enough contrast against white
/// for a camera to pick out QR modules drawn in it (yellows are the worst case).
Color qrInk(Color color) {
  final hsl = HSLColor.fromColor(color);
  return hsl.withLightness(hsl.lightness.clamp(0.0, 0.26)).toColor();
}

/// An app theme built around a signer's colour, so every screen of that signer wears it.
ThemeData signerTheme(String deviceId, Brightness brightness) => ThemeData(
      useMaterial3: true,
      colorScheme: ColorScheme.fromSeed(
        seedColor: signerColor(deviceId, brightness),
        brightness: brightness,
        // keep the primary close to the seed so buttons match the colour the coordinator shows
        dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
      ),
    );

/// A signer's name drawn in its own colour.
class SignerName extends StatelessWidget {
  const SignerName({super.key, required this.deviceId, required this.name, this.style});
  final String deviceId;
  final String name;
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final color = signerColor(deviceId, Theme.of(context).brightness);
    return Text(name, style: (style ?? const TextStyle()).copyWith(color: color, fontWeight: FontWeight.w600));
  }
}

/// A coloured pill for a signer, filled when it has replied and outlined while it is awaited.
class SignerChip extends StatelessWidget {
  const SignerChip({super.key, required this.deviceId, required this.name, this.done = false});
  final String deviceId;
  final String name;
  final bool done;

  @override
  Widget build(BuildContext context) {
    final color = signerColor(deviceId, Theme.of(context).brightness);
    return Chip(
      avatar: Icon(done ? Icons.check_circle : Icons.hourglass_empty, size: 18, color: color),
      label: Text(name, style: TextStyle(color: color, fontWeight: FontWeight.w600)),
      backgroundColor: done ? color.withValues(alpha: 0.18) : null,
      side: BorderSide(color: color, width: done ? 1 : 1.5),
      visualDensity: VisualDensity.compact,
    );
  }
}
