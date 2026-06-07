import 'dart:convert';

import 'package:flutter/material.dart';

import '../src/rust/api/chat_session.dart' as chat;
import '../src/rust/api/resource_catalog.dart' as catalog;

/// Bottom sheet for selecting ontologies and prior sessions bound to chat inference.
class ChatEnvironmentSheet extends StatefulWidget {
  final String sessionId;

  const ChatEnvironmentSheet({super.key, required this.sessionId});

  @override
  State<ChatEnvironmentSheet> createState() => _ChatEnvironmentSheetState();
}

class _ChatEnvironmentSheetState extends State<ChatEnvironmentSheet> {
  bool _loading = true;
  String? _error;
  List<String> _installedOntologyIds = [];
  List<catalog.OntologyResource> _catalogOntologies = [];
  final Set<String> _selectedOntologies = {};
  final Set<String> _selectedPriorSessions = {};
  List<chat.ChatSessionSummary> _allSessions = [];
  String _briefingPreview = '';

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
      final installed = await chat.listInstalledOntologyIdsForChat();
      final catalogOnts = await catalog.loadOntologyResources();
      final sessions = await chat.listChatSessions();
      final envJson = await chat.getSessionEnvironment(sessionId: widget.sessionId);
      final env = jsonDecode(envJson) as Map<String, dynamic>;

      final selected = (env['ontology_ids'] as List<dynamic>? ?? const [])
          .map((e) => e.toString())
          .toSet();
      final priors = (env['prior_session_ids'] as List<dynamic>? ?? const [])
          .map((e) => e.toString())
          .toSet();

      if (!mounted) return;
      setState(() {
        _installedOntologyIds = installed;
        _catalogOntologies = catalogOnts;
        _allSessions = sessions.where((s) => s.id != widget.sessionId).toList();
        _selectedOntologies
          ..clear()
          ..addAll(selected.isEmpty ? installed.toSet() : selected);
        _selectedPriorSessions
          ..clear()
          ..addAll(priors);
        _briefingPreview = env['capability_briefing'] as String? ?? '';
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

  String _ontologyLabel(String id) {
    for (final o in _catalogOntologies) {
      if (o.id == id) return o.name;
    }
    return id;
  }

  Future<void> _save() async {
    try {
      final envJson = await chat.updateSessionEnvironment(
        sessionId: widget.sessionId,
        ontologyIds: _selectedOntologies.toList(),
        priorSessionIds: _selectedPriorSessions.toList(),
      );
      final env = jsonDecode(envJson) as Map<String, dynamic>;
      if (!mounted) return;
      setState(() {
        _briefingPreview = env['capability_briefing'] as String? ?? '';
      });
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not save environment: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.72,
      minChildSize: 0.4,
      maxChildSize: 0.92,
      builder: (context, scrollController) {
        return Material(
          color: cs.surface,
          borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
          child: Column(
            children: [
              const SizedBox(height: 8),
              Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: cs.outlineVariant,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
                child: Row(
                  children: [
                    const Expanded(
                      child: Text(
                        'Chat environment',
                        style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                      ),
                    ),
                    TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
                    FilledButton(onPressed: _save, child: const Text('Apply')),
                  ],
                ),
              ),
              const Divider(height: 1),
              Expanded(
                child: _loading
                    ? const Center(child: CircularProgressIndicator())
                    : _error != null
                        ? Center(child: Text(_error!, style: TextStyle(color: cs.error)))
                        : ListView(
                            controller: scrollController,
                            padding: const EdgeInsets.all(16),
                            children: [
                              Text(
                                'Ontologies in scope',
                                style: Theme.of(context).textTheme.titleSmall,
                              ),
                              const SizedBox(height: 8),
                              if (_installedOntologyIds.isEmpty)
                                Text(
                                  'No installed ontologies — import from Ontology Hub first.',
                                  style: TextStyle(color: cs.onSurfaceVariant),
                                )
                              else
                                ..._installedOntologyIds.map((id) {
                                  return CheckboxListTile(
                                    value: _selectedOntologies.contains(id),
                                    title: Text(_ontologyLabel(id)),
                                    subtitle: Text(id, style: Theme.of(context).textTheme.bodySmall),
                                    onChanged: (v) {
                                      setState(() {
                                        if (v == true) {
                                          _selectedOntologies.add(id);
                                        } else {
                                          _selectedOntologies.remove(id);
                                        }
                                      });
                                    },
                                  );
                                }),
                              const SizedBox(height: 16),
                              Text(
                                'Link prior chats (optional)',
                                style: Theme.of(context).textTheme.titleSmall,
                              ),
                              const SizedBox(height: 8),
                              if (_allSessions.isEmpty)
                                Text(
                                  'No other saved sessions.',
                                  style: TextStyle(color: cs.onSurfaceVariant),
                                )
                              else
                                ..._allSessions.map((s) {
                                  return CheckboxListTile(
                                    value: _selectedPriorSessions.contains(s.id),
                                    title: Text(s.title),
                                    subtitle: Text('${s.messageCount} messages'),
                                    onChanged: (v) {
                                      setState(() {
                                        if (v == true) {
                                          _selectedPriorSessions.add(s.id);
                                        } else {
                                          _selectedPriorSessions.remove(s.id);
                                        }
                                      });
                                    },
                                  );
                                }),
                              const SizedBox(height: 16),
                              Text(
                                'LLM capability briefing (preview)',
                                style: Theme.of(context).textTheme.titleSmall,
                              ),
                              const SizedBox(height: 8),
                              Container(
                                width: double.infinity,
                                padding: const EdgeInsets.all(12),
                                decoration: BoxDecoration(
                                  color: cs.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Text(
                                  _briefingPreview.isEmpty
                                      ? 'Apply to compile environment briefing for the model.'
                                      : _briefingPreview,
                                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                        fontFamily: 'monospace',
                                      ),
                                ),
                              ),
                            ],
                          ),
              ),
            ],
          ),
        );
      },
    );
  }
}
