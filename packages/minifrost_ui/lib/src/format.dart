/// 1234567 -> "0.01234567 BTC"
String formatBtc(int sats) {
  final neg = sats < 0;
  final abs = sats.abs();
  final whole = abs ~/ 100000000;
  final frac = (abs % 100000000).toString().padLeft(8, '0');
  return '${neg ? '-' : ''}$whole.$frac BTC';
}

/// 1234567 -> "1 234 567 sats"
String formatSats(int sats) {
  final s = sats.abs().toString();
  final buf = StringBuffer();
  for (var i = 0; i < s.length; i++) {
    if (i > 0 && (s.length - i) % 3 == 0) buf.write(' ');
    buf.write(s[i]);
  }
  return '${sats < 0 ? '-' : ''}$buf sats';
}

/// Break a long address into groups so it can be compared by eye.
String groupAddress(String a, {int group = 4}) {
  final out = StringBuffer();
  for (var i = 0; i < a.length; i += group) {
    if (i > 0) out.write(' ');
    out.write(a.substring(i, i + group > a.length ? a.length : i + group));
  }
  return out.toString();
}
