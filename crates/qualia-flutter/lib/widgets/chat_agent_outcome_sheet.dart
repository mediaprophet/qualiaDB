import 'package:flutter/material.dart';

import '../src/rust/api/chat_agents.dart' as agents;

/// Configure how your sub-agent's processed outcomes are shared in a group chat.
class ChatAgentOutcomeSheet extends StatefulWidget {
  final String sessionId;
  final List<String> participantDids;

  const ChatAgentOutcomeSheet({
    super.key,
    required this.sessionId,
    this.participantDids = const [],
  });

  @override
  State<ChatAgentOutcomeSheet> createState() => _ChatAgentOutcomeSheetState();
}

class _ChatAgentOutcomeSheetState extends State<ChatAgentOutcomeSheet> {
  bool _loading = true;
  String _visibility = 'owner_only';
  bool _shareProvenance = true;
  bool _shareModelAttribution = false;
  bool _allowPeerLlmContext = false;
  final _allowedController = TextEditingController();
  String? _subAgentDid;
  String? _modelId;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _allowedController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final cfg = await agents.getLocalAgentConfig(sessionId: widget.sessionId);
      final p = cfg.outcomeSharing;
      if (!mounted) return;
      setState(() {
        _visibility = p.visibility;
        _shareProvenance = p.shareProvenance;
        _shareModelAttribution = p.shareModelAttribution;
        _allowPeerLlmContext = p.allowPeerLlmContext;
        _subAgentDid = cfg.subAgentDid;
        _modelId = cfg.modelId;
        if (p.allowedDids.isNotEmpty) {
          _allowedController.text = p.allowedDids.join(', ');
        }
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _loading = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Failed to load agent config: $e')),
      );
    }
  }

  agents.OutcomeSharingPolicy _buildPolicy() {
    final allowed = _allowedController.text
        .split(RegExp(r'[,\s]+'))
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    return agents.OutcomeSharingPolicy(
      visibility: _visibility,
      shareProvenance: _shareProvenance,
      shareModelAttribution: _shareModelAttribution,
      allowPeerLlmContext: _allowPeerLlmContext,
      allowedDids: allowed,
    );
  }

  Future<void> _save() async {
    try {
      await agents.updateAgentOutcomeSharing(
        sessionId: widget.sessionId,
        policy: _buildPolicy(),
      );
      if (!mounted) return;
      Navigator.pop(context, true);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Save failed: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
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
                Text(
                  'Sub-agent outcome sharing',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                Text(
                  'Your Webizen agent acts as a sub-agent of you—not an independent participant. '
                  'Choose whether processed outcomes (grounded answers, summaries) may be relayed to other group members.',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                if (_subAgentDid != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    'Sub-agent: $_subAgentDid',
                    style: Theme.of(context).textTheme.labelSmall,
                  ),
                ],
                if (_modelId != null) ...[
                  Text(
                    'Active model: $_modelId',
                    style: Theme.of(context).textTheme.labelSmall,
                  ),
                ],
                const SizedBox(height: 16),
                DropdownButtonFormField<String>(
                  value: _visibility,
                  decoration: const InputDecoration(
                    labelText: 'Share outcomes with',
                    border: OutlineInputBorder(),
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: 'owner_only',
                      child: Text('Only me (default)'),
                    ),
                    DropdownMenuItem(
                      value: 'session_participants',
                      child: Text('All group participants'),
                    ),
                    DropdownMenuItem(
                      value: 'specific_dids',
                      child: Text('Specific DIDs'),
                    ),
                  ],
                  onChanged: (v) {
                    if (v != null) setState(() => _visibility = v);
                  },
                ),
                if (_visibility == 'specific_dids') ...[
                  const SizedBox(height: 12),
                  TextField(
                    controller: _allowedController,
                    decoration: const InputDecoration(
                      labelText: 'Allowed participant DIDs',
                      hintText: 'did:qualia:…, did:qualia:…',
                      border: OutlineInputBorder(),
                    ),
                  ),
                ],
                const SizedBox(height: 12),
                SwitchListTile(
                  title: const Text('Include provenance / citations'),
                  subtitle: const Text('Share grounded citation hashes with peers'),
                  value: _shareProvenance,
                  onChanged: (v) => setState(() => _shareProvenance = v),
                ),
                SwitchListTile(
                  title: const Text('Attribute model'),
                  subtitle: const Text('Disclose which LLM produced the outcome'),
                  value: _shareModelAttribution,
                  onChanged: (v) => setState(() => _shareModelAttribution = v),
                ),
                SwitchListTile(
                  title: const Text('Allow peer LLM context'),
                  subtitle: const Text(
                    'Other participants\' agents may cite your shared outcomes',
                  ),
                  value: _allowPeerLlmContext,
                  onChanged: (v) => setState(() => _allowPeerLlmContext = v),
                ),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: _save,
                  child: const Text('Save'),
                ),
              ],
            ),
    );
  }
}
