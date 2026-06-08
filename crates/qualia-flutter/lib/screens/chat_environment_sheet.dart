import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../main.dart' show activeModelPathProvider, shellNavIndexProvider;
import '../src/rust/api/chat_session.dart' as chat;
import '../src/rust/api/qualia_api.dart' as api;
import '../src/rust/api/resource_catalog.dart' as catalog;

/// Bottom sheet for chat inference environment: model, ontologies, prior sessions.
class ChatEnvironmentSheet extends ConsumerStatefulWidget {
  final String sessionId;

  const ChatEnvironmentSheet({super.key, required this.sessionId});

  @override
  ConsumerState<ChatEnvironmentSheet> createState() =>
      _ChatEnvironmentSheetState();
}

class _ChatModelOption {
  final String id;
  final String label;
  final String? modality;

  const _ChatModelOption({
    required this.id,
    required this.label,
    this.modality,
  });
}

class _ChatEnvironmentSheetState extends ConsumerState<ChatEnvironmentSheet> {
  bool _loading = true;
  String? _error;
  bool _modelChanged = false;

  List<_ChatModelOption> _installedModels = [];
  String? _activeModelId;
  bool _activatingModel = false;

  List<String> _installedOntologyIds = [];
  List<catalog.OntologyResource> _catalogOntologies = [];
  final Set<String> _selectedOntologies = {};
  final Set<String> _selectedPriorSessions = {};
  List<chat.ChatSessionSummary> _allSessions = [];
  String _briefingPreview = '';
  bool _graphMutation = false;

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
      final installedIds = await catalog.listInstalledLlmIds();
      final llmCatalog = await catalog.loadLlmResources();
      final lifecycleJson = await catalog.getModelLifecycleStatus();
      final lifecycle = jsonDecode(lifecycleJson) as Map<String, dynamic>;
      final active = lifecycle['active'] as Map<String, dynamic>?;

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
      final graphMutation = env['graph_mutation'] as bool? ?? false;

      final models = <_ChatModelOption>[];
      for (final id in installedIds) {
        final match = llmCatalog.where((m) => m.id == id).toList();
        if (match.isNotEmpty) {
          final m = match.first;
          models.add(_ChatModelOption(
            id: id,
            label: m.name,
            modality: m.modality,
          ));
        } else {
          models.add(_ChatModelOption(
            id: id,
            label: id.replaceAll('-', ' '),
          ));
        }
      }
      models.sort((a, b) => a.label.compareTo(b.label));

      if (!mounted) return;
      setState(() {
        _installedModels = models;
        _activeModelId = active?['model_id'] as String?;
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
        _graphMutation = graphMutation;
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

  Future<void> _activateModel(String modelId) async {
    if (_activatingModel || _activeModelId == modelId) return;
    setState(() => _activatingModel = true);
    try {
      await api.setActiveModel(modelName: modelId);
      final path = await api.getActiveModel();
      if (path != null && path.isNotEmpty) {
        ref.read(activeModelPathProvider.notifier).state = path;
      }
      if (!mounted) return;
      setState(() {
        _activeModelId = modelId;
        _modelChanged = true;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Active model: $modelId')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not activate model: $e')),
      );
    } finally {
      if (mounted) setState(() => _activatingModel = false);
    }
  }

  Future<void> _applyLoadOrder() async {
    setState(() => _activatingModel = true);
    try {
      await catalog.applyModelPreference(task: 'chat');
      final path = await api.getActiveModel();
      if (path != null && path.isNotEmpty) {
        ref.read(activeModelPathProvider.notifier).state = path;
      }
      await _load();
      if (!mounted) return;
      setState(() => _modelChanged = true);
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Applied load order for chat')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Load order failed: $e')),
      );
    } finally {
      if (mounted) setState(() => _activatingModel = false);
    }
  }

  void _openLlmHub() {
    ref.read(shellNavIndexProvider.notifier).state = 7;
    Navigator.pop(context, _modelChanged);
  }

  Future<void> _save() async {
    try {
      final envJson = await chat.updateSessionEnvironment(
        sessionId: widget.sessionId,
        ontologyIds: _selectedOntologies.toList(),
        priorSessionIds: _selectedPriorSessions.toList(),
        graphMutation: _graphMutation,
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
      initialChildSize: 0.78,
      minChildSize: 0.45,
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
                    TextButton(
                      onPressed: () => Navigator.pop(context, _modelChanged),
                      child: const Text('Cancel'),
                    ),
                    FilledButton(onPressed: _save, child: const Text('Apply')),
                  ],
                ),
              ),
              const Divider(height: 1),
              Expanded(
                child: _loading
                    ? const Center(child: CircularProgressIndicator())
                    : _error != null
                        ? Center(
                            child: Text(_error!, style: TextStyle(color: cs.error)),
                          )
                        : ListView(
                            controller: scrollController,
                            padding: const EdgeInsets.all(16),
                            children: [
                              Text(
                                'Model for this chat',
                                style: Theme.of(context).textTheme.titleSmall,
                              ),
                              const SizedBox(height: 4),
                              Text(
                                'Pick an installed model or apply your LLM Hub load order.',
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(color: cs.onSurfaceVariant),
                              ),
                              const SizedBox(height: 8),
                              if (_installedModels.isEmpty)
                                Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      'No installed models yet.',
                                      style: TextStyle(color: cs.onSurfaceVariant),
                                    ),
                                    const SizedBox(height: 8),
                                    FilledButton.tonalIcon(
                                      onPressed: _openLlmHub,
                                      icon: const Icon(Icons.download_outlined),
                                      label: const Text('Open LLM Hub'),
                                    ),
                                  ],
                                )
                              else ...[
                                ..._installedModels.map((m) {
                                  final subtitle = m.modality == 'multimodal'
                                      ? '${m.id} · vision'
                                      : m.id;
                                  return RadioListTile<String>(
                                    value: m.id,
                                    groupValue: _activeModelId,
                                    title: Text(m.label),
                                    subtitle: Text(
                                      subtitle,
                                      style: Theme.of(context).textTheme.bodySmall,
                                    ),
                                    onChanged: _activatingModel
                                        ? null
                                        : (v) {
                                            if (v != null) _activateModel(v);
                                          },
                                  );
                                }),
                                const SizedBox(height: 4),
                                Wrap(
                                  spacing: 8,
                                  runSpacing: 4,
                                  children: [
                                    OutlinedButton.icon(
                                      onPressed:
                                          _activatingModel ? null : _applyLoadOrder,
                                      icon: const Icon(Icons.sort, size: 18),
                                      label: const Text('Apply load order'),
                                    ),
                                    TextButton.icon(
                                      onPressed: _openLlmHub,
                                      icon: const Icon(Icons.open_in_new, size: 18),
                                      label: const Text('LLM Hub'),
                                    ),
                                  ],
                                ),
                              ],
                              const SizedBox(height: 20),
                              Text(
                                'Neuro-symbolic output',
                                style: Theme.of(context).textTheme.titleSmall,
                              ),
                              SwitchListTile(
                                title: const Text('Graph mutation (sieve + WAL)'),
                                subtitle: Text(
                                  'Emit structured Super-Quins through the orchestrator instead of free-text prose.',
                                  style: Theme.of(context)
                                      .textTheme
                                      .bodySmall
                                      ?.copyWith(color: cs.onSurfaceVariant),
                                ),
                                value: _graphMutation,
                                onChanged: (v) => setState(() => _graphMutation = v),
                              ),
                              const SizedBox(height: 16),
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
                                    subtitle: Text(
                                      id,
                                      style: Theme.of(context).textTheme.bodySmall,
                                    ),
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
