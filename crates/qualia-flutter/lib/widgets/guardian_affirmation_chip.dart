import 'package:flutter/material.dart';

/// Amber pending or green ratified state for suspended WAL handoffs.
class GuardianAffirmationChip extends StatelessWidget {
  final bool walSuspended;
  final bool ratified;
  final int? agreementId;

  const GuardianAffirmationChip({
    super.key,
    required this.walSuspended,
    this.ratified = false,
    this.agreementId,
  });

  @override
  Widget build(BuildContext context) {
    if (!walSuspended && !ratified) return const SizedBox.shrink();

    final ratifiedState = ratified;
    final bg = ratifiedState
        ? Colors.green.withValues(alpha: 0.15)
        : Colors.amber.withValues(alpha: 0.15);
    final fg = ratifiedState ? Colors.green.shade300 : Colors.amber.shade300;
    final icon = ratifiedState ? Icons.verified_user : Icons.hourglass_top;
    final label = ratifiedState
        ? 'Cryptographically ratified'
        : 'Awaiting guardian affirmation';

    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: fg.withValues(alpha: 0.5)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 16, color: fg),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(color: fg, fontSize: 12, fontWeight: FontWeight.w500),
            ),
            if (agreementId != null && !ratifiedState) ...[
              const SizedBox(width: 8),
              Text(
                '0x${agreementId!.toRadixString(16).padLeft(8, '0')}',
                style: TextStyle(
                  color: fg.withValues(alpha: 0.8),
                  fontSize: 10,
                  fontFamily: 'monospace',
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
