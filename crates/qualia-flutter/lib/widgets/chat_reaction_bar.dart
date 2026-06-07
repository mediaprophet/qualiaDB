import 'package:flutter/material.dart';

import '../src/rust/api/chat_graph.dart' as graph;

/// Quick unicode emoji reactions under a chat message.
class ChatReactionBar extends StatelessWidget {
  static const quickEmojis = ['👍', '❤️', '😂', '❓', '✏️', '💡', '✅', '⚠️'];

  final String sessionId;
  final BigInt messageLamport;
  final List<graph.ChatReaction> reactions;
  final VoidCallback onChanged;

  const ChatReactionBar({
    super.key,
    required this.sessionId,
    required this.messageLamport,
    required this.reactions,
    required this.onChanged,
  });

  Future<void> _toggle(BuildContext context, String emoji) async {
    try {
      await graph.toggleChatReaction(
        sessionId: sessionId,
        messageLamport: messageLamport,
        emoji: emoji,
      );
      onChanged();
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Reaction failed: $e')),
        );
      }
    }
  }

  Map<String, int> get _counts {
    final counts = <String, int>{};
    for (final r in reactions) {
      counts[r.emoji] = (counts[r.emoji] ?? 0) + 1;
    }
    return counts;
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final counts = _counts;

    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Wrap(
        spacing: 2,
        runSpacing: 2,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          ...quickEmojis.map((emoji) {
            final count = counts[emoji] ?? 0;
            return InkWell(
              borderRadius: BorderRadius.circular(12),
              onTap: () => _toggle(context, emoji),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                child: Text(
                  count > 0 ? '$emoji $count' : emoji,
                  style: TextStyle(
                    fontSize: 14,
                    color: count > 0 ? cs.primary : cs.onSurfaceVariant,
                  ),
                ),
              ),
            );
          }),
          if (counts.entries.any((e) => !quickEmojis.contains(e.key)))
            ...counts.entries
                .where((e) => !quickEmojis.contains(e.key))
                .map(
                  (e) => Chip(
                    label: Text('${e.key} ${e.value}', style: const TextStyle(fontSize: 11)),
                    visualDensity: VisualDensity.compact,
                    padding: EdgeInsets.zero,
                  ),
                ),
        ],
      ),
    );
  }
}
