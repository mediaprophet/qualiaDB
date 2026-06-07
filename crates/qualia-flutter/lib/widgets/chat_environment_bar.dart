import 'dart:convert';

import 'package:flutter/material.dart';

import '../src/rust/api/chat_session.dart' as chat;
import '../src/rust/api/resource_catalog.dart' as catalog;

/// Compact status strip: model, ontologies, daemon, session.
class ChatEnvironmentBar extends StatelessWidget {
  final String? sessionId;
  final VoidCallback? onTap;

  const ChatEnvironmentBar({super.key, required this.sessionId, this.onTap});

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<_EnvSnapshot>(
      future: _load(sessionId),
      builder: (context, snap) {
        final data = snap.data;
        final cs = Theme.of(context).colorScheme;

        return Material(
          color: cs.surfaceContainerLow,
          child: InkWell(
            onTap: onTap,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              child: Row(
                children: [
                  Icon(Icons.hub_outlined, size: 16, color: cs.primary),
                  const SizedBox(width: 8),
                  Expanded(
                    child: snap.connectionState == ConnectionState.waiting
                        ? Text('Loading environment…', style: Theme.of(context).textTheme.bodySmall)
                        : Wrap(
                            spacing: 6,
                            runSpacing: 4,
                            children: [
                              _chip(
                                context,
                                icon: Icons.memory_outlined,
                                label: data?.modelLabel ?? 'No model',
                                highlight: data?.modelActive ?? false,
                              ),
                              _chip(
                                context,
                                icon: Icons.account_tree_outlined,
                                label: '${data?.ontologyCount ?? 0} ontologies',
                              ),
                              _chip(
                                context,
                                icon: Icons.cloud_done_outlined,
                                label: data?.daemonOk == true ? 'Daemon live' : 'Daemon off',
                                highlight: data?.daemonOk == true,
                              ),
                              if ((data?.qappCount ?? 0) > 0)
                                _chip(
                                  context,
                                  icon: Icons.apps_outlined,
                                  label: '${data!.qappCount} qapps',
                                ),
                            ],
                          ),
                  ),
                  if (onTap != null)
                    Icon(Icons.tune, size: 18, color: cs.onSurfaceVariant),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _chip(
    BuildContext context, {
    required IconData icon,
    required String label,
    bool highlight = false,
  }) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: highlight ? cs.primaryContainer : cs.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: highlight ? cs.onPrimaryContainer : cs.onSurfaceVariant),
          const SizedBox(width: 4),
          Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: highlight ? cs.onPrimaryContainer : cs.onSurfaceVariant,
                ),
          ),
        ],
      ),
    );
  }

  static Future<_EnvSnapshot> _load(String? sessionId) async {
    String modelLabel = 'No model';
    var modelActive = false;
    var ontologyCount = 0;
    var daemonOk = false;
    var qappCount = 0;

    try {
      final lifecycleJson = await catalog.getModelLifecycleStatus();
      final lifecycle = jsonDecode(lifecycleJson) as Map<String, dynamic>;
      modelActive = lifecycle['lifecycle_state'] == 'Active';
      final active = lifecycle['active'] as Map<String, dynamic>?;
      if (active != null) {
        modelLabel = active['model_id'] as String? ?? modelLabel;
        if (active['modality'] == 'multimodal') {
          modelLabel = '$modelLabel (VLM)';
        }
      }
    } catch (_) {}

    if (sessionId != null) {
      try {
        final envJson = await chat.getSessionEnvironment(sessionId: sessionId);
        final env = jsonDecode(envJson) as Map<String, dynamic>;
        ontologyCount = (env['ontology_ids'] as List<dynamic>? ?? const []).length;
        daemonOk = env['daemon_reachable'] == true;
        qappCount = (env['installed_qapps'] as List<dynamic>? ?? const []).length;
      } catch (_) {}
    }

    return _EnvSnapshot(
      modelLabel: modelLabel,
      modelActive: modelActive,
      ontologyCount: ontologyCount,
      daemonOk: daemonOk,
      qappCount: qappCount,
    );
  }
}

class _EnvSnapshot {
  final String modelLabel;
  final bool modelActive;
  final int ontologyCount;
  final bool daemonOk;
  final int qappCount;

  const _EnvSnapshot({
    required this.modelLabel,
    required this.modelActive,
    required this.ontologyCount,
    required this.daemonOk,
    required this.qappCount,
  });
}
