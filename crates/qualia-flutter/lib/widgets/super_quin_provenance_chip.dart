import 'package:flutter/material.dart';

import 'super_quin_inspector_sheet.dart';

/// Authoritative sieve output: Subject → Predicate → Object with optional WAL badge.
class SuperQuinProvenanceChip extends StatelessWidget {
  final List<int> fields;
  final bool walCommitted;
  final int sieveTokenCount;

  const SuperQuinProvenanceChip({
    super.key,
    required this.fields,
    this.walCommitted = false,
    this.sieveTokenCount = 0,
  });

  String _hex(int v) => '0x${v.toRadixString(16).padLeft(16, '0')}';

  @override
  Widget build(BuildContext context) {
    if (fields.length < 3) return const SizedBox.shrink();

    final cs = Theme.of(context).colorScheme;
    final subject = _hex(fields[0]);
    final predicate = _hex(fields[1]);
    final object = _hex(fields[2]);

    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Material(
        color: cs.tertiaryContainer.withValues(alpha: 0.55),
        borderRadius: BorderRadius.circular(10),
        child: InkWell(
          borderRadius: BorderRadius.circular(10),
          onTap: () => showModalBottomSheet<void>(
            context: context,
            isScrollControlled: true,
            showDragHandle: true,
            builder: (_) => SuperQuinInspectorSheet(
              fields: fields,
              walCommitted: walCommitted,
              sieveTokenCount: sieveTokenCount,
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            child: Row(
              children: [
                Icon(Icons.hub_outlined, size: 18, color: cs.onTertiaryContainer),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '$subject → $predicate → $object',
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                          fontFamily: 'monospace',
                          color: cs.onTertiaryContainer,
                        ),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (walCommitted) ...[
                  const SizedBox(width: 6),
                  Icon(Icons.lock_outline, size: 16, color: cs.primary),
                  const SizedBox(width: 4),
                  Text(
                    'Ledger',
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: cs.primary,
                          fontWeight: FontWeight.w600,
                        ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
