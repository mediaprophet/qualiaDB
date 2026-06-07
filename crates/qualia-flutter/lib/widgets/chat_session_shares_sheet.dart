import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../src/rust/api/chat_session.dart' as chat;
import '../src/rust/api/ontology_workbench.dart' as wb;

/// Ontology magnets shared with this chat session (scoped by `session_did`).
class ChatSessionSharesSheet extends StatefulWidget {
  final String sessionId;

  const ChatSessionSharesSheet({super.key, required this.sessionId});

  @override
  State<ChatSessionSharesSheet> createState() => _ChatSessionSharesSheetState();
}

class _ChatSessionSharesSheetState extends State<ChatSessionSharesSheet> {
  String? _sessionDid;
  List<wb.OntologyShareCard> _cards = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final did = await chat.getChatSessionDid(sessionId: widget.sessionId);
      final cards = await wb.listOntologySharesForSession(sessionDid: did);
      if (!mounted) return;
      setState(() {
        _sessionDid = did;
        _cards = cards;
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

  Future<void> _postMagnet(wb.OntologyShareCard card) async {
    final did = _sessionDid;
    if (did == null) return;
    final body = [
      '**Shared ontology:** ${card.title}',
      'Domain: ${card.domain} · ${card.quinCount} quins',
      'Session DID: `$did`',
      'Magnet: ${card.magnetUri}',
    ].join('\n');
    try {
      await chat.appendChatMessage(
        sessionId: widget.sessionId,
        role: 'assistant',
        content: body,
      );
      if (!mounted) return;
      Navigator.pop(context, true);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Posted ${card.title} magnet to chat')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Failed to post: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: EdgeInsets.only(
        left: 16,
        right: 16,
        top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: _loading
          ? const Center(child: CircularProgressIndicator())
          : Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text('Shared ontologies', style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 4),
                Text(
                  'Ontologies whose workbench policy includes this chat\'s session DID.',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: cs.onSurfaceVariant,
                      ),
                ),
                if (_sessionDid != null) ...[
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      Expanded(
                        child: SelectableText(
                          _sessionDid!,
                          style: const TextStyle(fontSize: 11, fontFamily: 'monospace'),
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.copy, size: 18),
                        tooltip: 'Copy session DID',
                        onPressed: () {
                          Clipboard.setData(ClipboardData(text: _sessionDid!));
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('Session DID copied')),
                          );
                        },
                      ),
                    ],
                  ),
                ],
                const SizedBox(height: 12),
                if (_error != null)
                  Text(_error!, style: TextStyle(color: cs.error))
                else if (_cards.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 16),
                    child: Text(
                      'No ontologies are shared with this session yet. '
                      'Open Ontology Hub → workbench and choose “Chat sessions” as the audience.',
                      style: TextStyle(color: cs.onSurfaceVariant),
                    ),
                  )
                else
                  ConstrainedBox(
                    constraints: BoxConstraints(
                      maxHeight: MediaQuery.of(context).size.height * 0.45,
                    ),
                    child: ListView.builder(
                      shrinkWrap: true,
                      itemCount: _cards.length,
                      itemBuilder: (context, i) {
                        final c = _cards[i];
                        return Card(
                          margin: const EdgeInsets.only(bottom: 8),
                          child: ListTile(
                            title: Text(c.title),
                            subtitle: Text(
                              '${c.domain} · ${c.quinCount} quins',
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                            ),
                            trailing: PopupMenuButton<String>(
                              onSelected: (action) {
                                if (action == 'copy') {
                                  Clipboard.setData(ClipboardData(text: c.magnetUri));
                                  ScaffoldMessenger.of(context).showSnackBar(
                                    const SnackBar(content: Text('Magnet copied')),
                                  );
                                } else if (action == 'post') {
                                  _postMagnet(c);
                                }
                              },
                              itemBuilder: (_) => const [
                                PopupMenuItem(value: 'copy', child: Text('Copy magnet')),
                                PopupMenuItem(value: 'post', child: Text('Post to chat')),
                              ],
                            ),
                          ),
                        );
                      },
                    ),
                  ),
              ],
            ),
    );
  }
}
