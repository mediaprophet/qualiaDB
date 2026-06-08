import 'package:flutter/material.dart';

import 'sensitivity_badge.dart';

/// Six-field Super-Quin inspector (power-user / provenance drill-down).
class SuperQuinInspectorSheet extends StatelessWidget {
  final List<int> fields;
  final bool walCommitted;
  final int sieveTokenCount;
  final String? principalLabel;

  const SuperQuinInspectorSheet({
    super.key,
    required this.fields,
    this.walCommitted = false,
    this.sieveTokenCount = 0,
    this.principalLabel,
  });

  static const _labels = [
    'subject',
    'predicate',
    'object',
    'context',
    'metadata',
    'parity',
  ];

  String _hex(int v) => '0x${v.toRadixString(16).padLeft(16, '0')}';

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final contextField = fields.length > 3 ? fields[3] : null;

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Super-Quin inspector',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            if (walCommitted)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Chip(
                  avatar: Icon(Icons.verified_outlined, size: 18, color: cs.primary),
                  label: const Text('Ledger committed'),
                ),
              ),
            if (sieveTokenCount > 0)
              Text(
                'Sieve tokens: $sieveTokenCount',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 6,
              runSpacing: 4,
              children: [
                SensitivityBadge(contextField: contextField),
                if (principalLabel != null && principalLabel!.isNotEmpty)
                  Chip(
                    label: Text('Principal: $principalLabel'),
                    visualDensity: VisualDensity.compact,
                  ),
                Chip(
                  label: const Text('Signature: Ed25519 stub'),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            const SizedBox(height: 12),
            ...List.generate(fields.length.clamp(0, 6), (i) {
              return Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: SelectableText(
                  '${_labels[i]}: ${_hex(fields[i])}',
                  style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                ),
              );
            }),
          ],
        ),
      ),
    );
  }
}
