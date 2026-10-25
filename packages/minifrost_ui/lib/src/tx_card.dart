import 'package:flutter/material.dart';

import 'format.dart';

class TxOutputRow {
  const TxOutputRow({required this.address, required this.sats, this.ownLabel});
  final String address;
  final int sats;
  final String? ownLabel;
}

/// A transaction as the user must see it before approving anything.
class TxCard extends StatelessWidget {
  const TxCard({
    super.key,
    required this.outputs,
    required this.feeSats,
    this.feeRate,
    this.nInputs,
    this.txid,
    this.warnings = const [],
    this.verified = false,
  });

  final List<TxOutputRow> outputs;
  final int feeSats;
  final double? feeRate;
  final int? nInputs;
  final String? txid;
  final List<String> warnings;
  final bool verified;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('Sending', style: t.textTheme.labelLarge),
          const SizedBox(height: 8),
          for (final o in outputs) ...[
            Text(formatBtc(o.sats), style: t.textTheme.headlineSmall),
            Text(formatSats(o.sats), style: t.textTheme.bodySmall),
            const SizedBox(height: 4),
            if (o.ownLabel != null)
              Chip(label: Text('Back to this wallet · ${o.ownLabel}'), visualDensity: VisualDensity.compact),
            if (o.address.isNotEmpty)
              SelectableText(groupAddress(o.address), style: const TextStyle(fontFamily: 'monospace', fontSize: 15)),
            const Divider(height: 24),
          ],
          Row(children: [
            Text('Network fee: ', style: t.textTheme.bodyMedium),
            Text(formatSats(feeSats), style: t.textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.bold)),
            if (feeRate != null) Text('  (${feeRate!.toStringAsFixed(1)} sat/vB)'),
          ]),
          if (nInputs != null) Text('Inputs: $nInputs'),
          if (txid != null) ...[
            const SizedBox(height: 8),
            SelectableText('txid ${txid!}', style: const TextStyle(fontFamily: 'monospace', fontSize: 11)),
          ],
          const SizedBox(height: 8),
          Row(children: [
            Icon(verified ? Icons.verified_user : Icons.info_outline,
                size: 18, color: verified ? Colors.green : t.colorScheme.tertiary),
            const SizedBox(width: 6),
            Expanded(
                child: Text(
              verified
                  ? 'Checked against this signer’s own key.'
                  : 'Not yet checked against this signer’s key — shown as the coordinator presents it.',
              style: t.textTheme.bodySmall,
            )),
          ]),
          for (final w in warnings)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Icon(Icons.warning_amber, size: 18, color: t.colorScheme.error),
                const SizedBox(width: 6),
                Expanded(child: Text(w, style: TextStyle(color: t.colorScheme.error))),
              ]),
            ),
        ]),
      ),
    );
  }
}
