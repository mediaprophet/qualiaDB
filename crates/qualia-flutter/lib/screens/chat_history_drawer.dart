import 'package:flutter/material.dart';

import '../src/rust/api/chat_session.dart' as chat;

/// Sidebar drawer listing persisted chat sessions.
class ChatHistoryDrawer extends StatefulWidget {
  final String? activeSessionId;
  final ValueChanged<String> onSessionSelected;
  final VoidCallback onNewChat;

  const ChatHistoryDrawer({
    super.key,
    required this.activeSessionId,
    required this.onSessionSelected,
    required this.onNewChat,
  });

  @override
  State<ChatHistoryDrawer> createState() => _ChatHistoryDrawerState();
}

class _ChatHistoryDrawerState extends State<ChatHistoryDrawer> {
  List<chat.ChatSessionSummary> _sessions = [];
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
      final sessions = await chat.listChatSessions();
      if (!mounted) return;
      setState(() {
        _sessions = sessions;
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

  Future<void> _deleteSession(chat.ChatSessionSummary session) async {
    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete chat?'),
        content: Text('Remove "${session.title}" permanently?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Delete')),
        ],
      ),
    );
    if (confirm != true) return;

    try {
      await chat.deleteChatSession(sessionId: session.id);
      if (!mounted) return;
      if (widget.activeSessionId == session.id) {
        widget.onNewChat();
      }
      await _reload();
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Delete failed: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Drawer(
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 16, 8, 8),
              child: Row(
                children: [
                  const Expanded(
                    child: Text('Chat history', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
                  ),
                  IconButton(
                    tooltip: 'New chat',
                    icon: const Icon(Icons.add_comment_outlined),
                    onPressed: () {
                      widget.onNewChat();
                      Navigator.pop(context);
                    },
                  ),
                  IconButton(
                    tooltip: 'Refresh',
                    icon: const Icon(Icons.refresh),
                    onPressed: _reload,
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: _loading
                  ? const Center(child: CircularProgressIndicator())
                  : _error != null
                      ? Center(child: Text(_error!, style: TextStyle(color: cs.error)))
                      : _sessions.isEmpty
                          ? const Center(child: Text('No saved chats yet'))
                          : ListView.builder(
                              itemCount: _sessions.length,
                              itemBuilder: (context, i) {
                                final s = _sessions[i];
                                final selected = s.id == widget.activeSessionId;
                                return Dismissible(
                                  key: ValueKey(s.id),
                                  direction: DismissDirection.endToStart,
                                  background: Container(
                                    color: cs.errorContainer,
                                    alignment: Alignment.centerRight,
                                    padding: const EdgeInsets.only(right: 20),
                                    child: Icon(Icons.delete_outline, color: cs.onErrorContainer),
                                  ),
                                  confirmDismiss: (_) async {
                                    await _deleteSession(s);
                                    return false;
                                  },
                                  child: ListTile(
                                    selected: selected,
                                    title: Text(
                                      s.title,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                    subtitle: Text(
                                      '${s.messageCount} message${s.messageCount == BigInt.one ? '' : 's'}',
                                      style: Theme.of(context).textTheme.bodySmall,
                                    ),
                                    onTap: () {
                                      widget.onSessionSelected(s.id);
                                      Navigator.pop(context);
                                    },
                                  ),
                                );
                              },
                            ),
            ),
          ],
        ),
      ),
    );
  }
}
