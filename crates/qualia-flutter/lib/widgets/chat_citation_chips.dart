import 'package:flutter/material.dart';

class ChatCitation {
  final String ontologyId;
  final String label;

  const ChatCitation({required this.ontologyId, required this.label});
}

/// Grounding citation chips shown under agent messages.
class ChatCitationChips extends StatelessWidget {
  final List<ChatCitation> citations;
  final int provenanceCount;
  final bool committed;

  const ChatCitationChips({
    super.key,
    required this.citations,
    required this.provenanceCount,
    required this.committed,
  });

  @override
  Widget build(BuildContext context) {
    if (!committed && citations.isEmpty && provenanceCount == 0) {
      return const SizedBox.shrink();
    }

    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Wrap(
        spacing: 6,
        runSpacing: 4,
        children: [
          if (committed)
            _chip(
              context,
              icon: Icons.verified_outlined,
              label: '$provenanceCount provenance',
              color: cs.tertiaryContainer,
              onColor: cs.onTertiaryContainer,
            )
          else
            _chip(
              context,
              icon: Icons.gpp_maybe_outlined,
              label: 'Ungrounded',
              color: cs.errorContainer,
              onColor: cs.onErrorContainer,
            ),
          ...citations.take(6).map(
                (c) => _chip(
                  context,
                  icon: Icons.link,
                  label: '${c.ontologyId}: ${c.label}',
                  color: cs.surfaceContainerHigh,
                  onColor: cs.onSurfaceVariant,
                ),
              ),
          if (citations.length > 6)
            _chip(
              context,
              icon: Icons.more_horiz,
              label: '+${citations.length - 6} more',
              color: cs.surfaceContainerHigh,
              onColor: cs.onSurfaceVariant,
            ),
        ],
      ),
    );
  }

  Widget _chip(
    BuildContext context, {
    required IconData icon,
    required String label,
    required Color color,
    required Color onColor,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: onColor),
          const SizedBox(width: 4),
          Flexible(
            child: Text(
              label,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(color: onColor),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
