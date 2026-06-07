import 'package:flutter/material.dart';

import '../src/rust/api/chat_graph.dart' as graph;
import '../src/rust/api/chat_session.dart' as chat;

/// Visualizes chat fragments and reply edges as a branching graph.
class ChatGraphPanel extends StatefulWidget {
  final String sessionId;

  const ChatGraphPanel({super.key, required this.sessionId});

  @override
  State<ChatGraphPanel> createState() => _ChatGraphPanelState();
}

class _ChatGraphPanelState extends State<ChatGraphPanel> {
  graph.ChatGraphView? _graph;
  List<chat.ChatMessage> _messages = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final g = await graph.getChatGraph(sessionId: widget.sessionId);
      final msgs = await chat.loadChatSessionMessages(id: widget.sessionId);
      if (!mounted) return;
      setState(() {
        _graph = g;
        _messages = msgs;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  String _messagePreview(BigInt lamport) {
    for (final m in _messages) {
      if (m.lamport == lamport) {
        final text = m.content.replaceAll('\n', ' ');
        return text.length > 80 ? '${text.substring(0, 80)}…' : text;
      }
    }
    return 'msg #$lamport';
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Expanded(
                child: Text('Chat graph', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
              ),
              IconButton(icon: const Icon(Icons.refresh), onPressed: _reload),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'Branches show which sentence or clause each reply addresses.',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 12),
          Expanded(
            child: _loading
                ? const Center(child: CircularProgressIndicator())
                : _error != null
                    ? Center(child: Text(_error!, style: TextStyle(color: cs.error)))
                    : _buildGraphBody(cs),
          ),
        ],
      ),
    );
  }

  Widget _buildGraphBody(ColorScheme cs) {
    final g = _graph!;
    if (g.fragments.isEmpty) {
      return const Center(child: Text('No fragments yet — select text in a message to start a branch.'));
    }

    final roots = g.fragments.where((f) {
      return !g.edges.any((e) => e.childFragmentId == f.fragmentId);
    }).toList();

    return ListView(
      children: [
        for (final root in roots) _buildNode(root, g, cs, depth: 0),
      ],
    );
  }

  Widget _buildNode(graph.ChatFragment node, graph.ChatGraphView g, ColorScheme cs, {required int depth}) {
    final children = g.edges
        .where((e) => e.parentFragmentId == node.fragmentId)
        .map((e) {
          final match = g.fragments.where((f) => f.fragmentId == e.childFragmentId);
          if (match.isNotEmpty) return match.first;
          return graph.ChatFragment(
            fragmentId: e.childFragmentId,
            messageLamport: e.replyMessageLamport,
            anchorStart: 0,
            anchorEnd: 0,
            anchorText: _messagePreview(e.replyMessageLamport),
            authorDid: null,
            authorName: null,
            createdAt: e.createdAt,
          );
        })
        .toList();

    return Padding(
      padding: EdgeInsets.only(left: depth * 16.0, bottom: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: cs.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: cs.outlineVariant),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(node.anchorText, style: const TextStyle(fontWeight: FontWeight.w600)),
                const SizedBox(height: 4),
                Builder(builder: (context) {
                  final edge = g.edges.cast<graph.ChatGraphEdge?>().firstWhere(
                        (e) => e?.childFragmentId == node.fragmentId,
                        orElse: () => null,
                      );
                  if (edge?.branchLabel != null) {
                    return Padding(
                      padding: const EdgeInsets.only(bottom: 4),
                      child: Chip(
                        label: Text(
                          '${edge!.branchEmoji ?? '💬'} ${edge.branchLabel}',
                          style: const TextStyle(fontSize: 11),
                        ),
                        visualDensity: VisualDensity.compact,
                      ),
                    );
                  }
                  return const SizedBox.shrink();
                }),
                Text(
                  'msg #${node.messageLamport} · ${node.fragmentId.length > 8 ? node.fragmentId.substring(0, 8) : node.fragmentId}…',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                if (node.authorName != null)
                  Text(node.authorName!, style: Theme.of(context).textTheme.bodySmall),
              ],
            ),
          ),
          for (final child in children) _buildNode(child, g, cs, depth: depth + 1),
        ],
      ),
    );
  }
}
