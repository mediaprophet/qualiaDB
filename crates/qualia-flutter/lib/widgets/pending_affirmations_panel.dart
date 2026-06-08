import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/pending_affirmations_service.dart';
import '../src/rust/api/qualia_api.dart' as api;

/// Slide-over panel for bilateral guardianship co-signature (CRC Art. 3 / Art. 12 framing).
class PendingAffirmationsPanel extends ConsumerWidget {
  final String? principalDid;
  final VoidCallback onClose;

  const PendingAffirmationsPanel({
    super.key,
    required this.onClose,
    this.principalDid,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pending = ref.watch(pendingAffirmationsProvider);
    final cs = Theme.of(context).colorScheme;

    return Material(
      elevation: 8,
      color: cs.surface,
      child: SizedBox(
        width: 360,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 16, 8, 8),
              child: Row(
                children: [
                  Icon(Icons.family_restroom, color: cs.secondary, size: 22),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Pending Affirmations',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: onClose,
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Text(
                'Co-signature required before graph mutations affecting dependents '
                'can be committed. Best interests of the child (CRC Art. 3); '
                'participation where minors are involved (CRC Art. 12).',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: cs.onSurfaceVariant,
                    ),
              ),
            ),
            const SizedBox(height: 12),
            Expanded(
              child: pending.isEmpty
                  ? Center(
                      child: Text(
                        'No pending guardianship proposals.',
                        style: TextStyle(color: cs.onSurfaceVariant),
                      ),
                    )
                  : ListView.builder(
                      padding: const EdgeInsets.symmetric(horizontal: 12),
                      itemCount: pending.length,
                      itemBuilder: (context, i) => _AffirmationCard(
                        tx: pending[i],
                        principalDid: principalDid,
                        onAction: () => ref.read(pendingAffirmationsProvider.notifier).poll(),
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AffirmationCard extends StatefulWidget {
  final api.SuspendedTxView tx;
  final String? principalDid;
  final VoidCallback onAction;

  const _AffirmationCard({
    required this.tx,
    required this.onAction,
    this.principalDid,
  });

  @override
  State<_AffirmationCard> createState() => _AffirmationCardState();
}

class _AffirmationCardState extends State<_AffirmationCard> {
  bool _busy = false;

  Future<void> _cosign() async {
    setState(() => _busy = true);
    try {
      final did = widget.principalDid ?? 'did:qualia:local-principal';
      await api.cosignPendingAffirmation(
        agreementId: widget.tx.agreementId,
        principalDid: did,
      );
      widget.onAction();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _deny() async {
    setState(() => _busy = true);
    try {
      await api.denyGuardianAffirmation(agreementId: widget.tx.agreementId);
      widget.onAction();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final tx = widget.tx;

    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      color: Colors.amber.withValues(alpha: 0.08),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              tx.label,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 6),
            Text(
              'Signatures ${tx.collectedSignatures} / ${tx.threshold}',
              style: TextStyle(
                fontSize: 12,
                color: cs.onSurfaceVariant,
                fontFamily: 'monospace',
              ),
            ),
            Text(
              'Agreement 0x${tx.agreementId.toRadixString(16).padLeft(16, '0')}',
              style: TextStyle(
                fontSize: 11,
                color: cs.onSurfaceVariant,
                fontFamily: 'monospace',
              ),
            ),
            const SizedBox(height: 10),
            Row(
              children: [
                FilledButton.icon(
                  onPressed: _busy ? null : _cosign,
                  icon: const Icon(Icons.draw, size: 18),
                  label: const Text('Co-sign'),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  onPressed: _busy ? null : _deny,
                  child: const Text('Deny'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
