import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../src/rust/api/chat_session.dart' as chat;
import '../src/rust/api/ontology_workbench.dart' as wb;
import '../src/rust/api/social_api.dart' as social;

/// URI → `.c.q42` workbench with WebTorrent magnet and sharing policy.
class OntologyWorkbenchSheet extends StatefulWidget {
  const OntologyWorkbenchSheet({super.key});

  @override
  State<OntologyWorkbenchSheet> createState() => _OntologyWorkbenchSheetState();
}

class _OntologyWorkbenchSheetState extends State<OntologyWorkbenchSheet> {
  final _uriController = TextEditingController();
  final _idController = TextEditingController();
  final _titleController = TextEditingController();
  final _domainController = TextEditingController(text: 'general');
  final _categoriesController = TextEditingController();
  final _didsController = TextEditingController();

  List<wb.WorkbenchEntry> _entries = [];
  List<social.ChatContact> _contacts = [];
  List<chat.ChatSessionShareTarget> _sessionTargets = [];
  wb.TorrentBandwidthPolicy? _bandwidth;
  bool _loading = true;
  bool _importing = false;
  String? _selectedId;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  @override
  void dispose() {
    _uriController.dispose();
    _idController.dispose();
    _titleController.dispose();
    _domainController.dispose();
    _categoriesController.dispose();
    _didsController.dispose();
    super.dispose();
  }

  Future<void> _refresh() async {
    setState(() => _loading = true);
    try {
      final entries = await wb.listWorkbenchOntologies();
      final contacts = await social.listChatContacts();
      final sessions = await chat.listChatSessionShareTargets();
      final bw = await wb.getTorrentBandwidthPolicy();
      if (!mounted) return;
      setState(() {
        _entries = entries;
        _contacts = contacts;
        _sessionTargets = sessions;
        _bandwidth = bw;
        _loading = false;
      });
    } catch (e) {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _import() async {
    final uri = _uriController.text.trim();
    if (uri.isEmpty) return;
    setState(() => _importing = true);
    try {
      final result = await wb.workbenchImportOntologyUri(
        uri: uri,
        ontologyId: _idController.text.trim().isEmpty ? null : _idController.text.trim(),
        domain: _domainController.text.trim().isEmpty ? null : _domainController.text.trim(),
        title: _titleController.text.trim().isEmpty ? null : _titleController.text.trim(),
      );
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            'Imported ${result.entry.title} → .c.q42 (${result.entry.quinCount} quins)',
          ),
        ),
      );
      _uriController.clear();
      await _refresh();
      setState(() => _selectedId = result.entry.ontologyId);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Import failed: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _importing = false);
    }
  }

  wb.WorkbenchEntry? get _selected {
    if (_selectedId == null) return null;
    for (final e in _entries) {
      if (e.ontologyId == _selectedId) return e;
    }
    return null;
  }

  Future<void> _savePolicy(wb.WorkbenchEntry entry, wb.OntologyTorrentPolicy policy) async {
    try {
      await wb.setWorkbenchTorrentPolicy(
        ontologyId: entry.ontologyId,
        policy: policy,
      );
      await _refresh();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Policy save failed: $e')),
        );
      }
    }
  }

  Future<void> _toggleSeed(wb.WorkbenchEntry entry, bool on) async {
    try {
      await wb.setWorkbenchSeed(ontologyId: entry.ontologyId, active: on);
      await _refresh();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Seed toggle failed: $e')),
        );
      }
    }
  }

  Future<void> _saveBandwidth(int kbps, bool metered) async {
    try {
      await wb.setTorrentBandwidthPolicy(
        policy: wb.TorrentBandwidthPolicy(
          globalLimitKbps: kbps,
          meteredMode: metered,
        ),
      );
      await _refresh();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Bandwidth save failed: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final selected = _selected;
    final bw = _bandwidth;

    return Padding(
      padding: EdgeInsets.only(
        left: 16,
        right: 16,
        top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: _loading
          ? const Center(child: CircularProgressIndicator())
          : SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text('Ontology workbench', style: Theme.of(context).textTheme.titleLarge),
                  const SizedBox(height: 4),
                  Text(
                    'Fetch RDF from a URI → compile `.c.q42` → magnet for WebTorrent sharing.',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: cs.onSurfaceVariant,
                        ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _uriController,
                    decoration: const InputDecoration(
                      labelText: 'Ontology URI',
                      hintText: 'https://example.org/vocab/ontology.ttl',
                      border: OutlineInputBorder(),
                    ),
                    maxLines: 2,
                  ),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _idController,
                          decoration: const InputDecoration(
                            labelText: 'ID (optional)',
                            border: OutlineInputBorder(),
                            isDense: true,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          controller: _domainController,
                          decoration: const InputDecoration(
                            labelText: 'Domain / category',
                            border: OutlineInputBorder(),
                            isDense: true,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _titleController,
                    decoration: const InputDecoration(
                      labelText: 'Title (optional)',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                  ),
                  const SizedBox(height: 12),
                  FilledButton.icon(
                    onPressed: _importing ? null : _import,
                    icon: _importing
                        ? const SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.cloud_download),
                    label: Text(_importing ? 'Processing…' : 'Import & compress'),
                  ),
                  if (bw != null) ...[
                    const Divider(height: 32),
                    Text('Global bandwidth', style: Theme.of(context).textTheme.titleSmall),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        Expanded(
                          child: Slider(
                            value: bw.globalLimitKbps.clamp(0, 8192).toDouble(),
                            min: 0,
                            max: 8192,
                            divisions: 32,
                            label: bw.globalLimitKbps == 0
                                ? 'unlimited'
                                : '${bw.globalLimitKbps} KiB/s',
                            onChanged: (v) => _saveBandwidth(v.round(), bw.meteredMode),
                          ),
                        ),
                        Text(
                          bw.globalLimitKbps == 0 ? '∞' : '${bw.globalLimitKbps}K',
                          style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                        ),
                      ],
                    ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: const Text('Metered mode'),
                      subtitle: const Text('Respect upload caps on cellular / limited plans'),
                      value: bw.meteredMode,
                      onChanged: (v) => _saveBandwidth(bw.globalLimitKbps, v),
                    ),
                  ],
                  const Divider(height: 32),
                  Text('Workbench (${_entries.length})', style: Theme.of(context).textTheme.titleSmall),
                  const SizedBox(height: 8),
                  if (_entries.isEmpty)
                    Text(
                      'No custom ontologies yet.',
                      style: TextStyle(color: cs.onSurfaceVariant),
                    )
                  else
                    ..._entries.map((e) {
                      final isSel = e.ontologyId == _selectedId;
                      return Card(
                        margin: const EdgeInsets.only(bottom: 8),
                        color: isSel ? cs.primaryContainer.withValues(alpha: 0.35) : null,
                        child: ListTile(
                          title: Text(e.title),
                          subtitle: Text(
                            '${e.domain} · ${e.quinCount} quins · ${e.seedActive ? "seeding" : "idle"}',
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ),
                          trailing: IconButton(
                            icon: const Icon(Icons.link),
                            tooltip: 'Copy magnet',
                            onPressed: () {
                              Clipboard.setData(ClipboardData(text: e.magnetUri));
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(content: Text('Magnet URI copied')),
                              );
                            },
                          ),
                          onTap: () => setState(() => _selectedId = e.ontologyId),
                        ),
                      );
                    }),
                  if (selected != null) ...[
                    const Divider(height: 24),
                    Text('Sharing: ${selected.title}', style: Theme.of(context).textTheme.titleSmall),
                    const SizedBox(height: 8),
                    _SharingEditor(
                      entry: selected,
                      contacts: _contacts,
                      sessionTargets: _sessionTargets,
                      categoriesController: _categoriesController,
                      didsController: _didsController,
                      onSave: _savePolicy,
                      onToggleSeed: _toggleSeed,
                    ),
                  ],
                ],
              ),
            ),
    );
  }
}

class _SharingEditor extends StatefulWidget {
  final wb.WorkbenchEntry entry;
  final List<social.ChatContact> contacts;
  final List<chat.ChatSessionShareTarget> sessionTargets;
  final TextEditingController categoriesController;
  final TextEditingController didsController;
  final Future<void> Function(wb.WorkbenchEntry, wb.OntologyTorrentPolicy) onSave;
  final Future<void> Function(wb.WorkbenchEntry, bool) onToggleSeed;

  const _SharingEditor({
    required this.entry,
    required this.contacts,
    required this.sessionTargets,
    required this.categoriesController,
    required this.didsController,
    required this.onSave,
    required this.onToggleSeed,
  });

  @override
  State<_SharingEditor> createState() => _SharingEditorState();
}

class _SharingEditorState extends State<_SharingEditor> {
  late String _audience;
  late bool _shareEnabled;
  late bool _seedEnabled;
  late double _bwLimit;
  final Set<String> _selectedSessionDids = {};

  @override
  void initState() {
    super.initState();
    final t = widget.entry.torrent;
    _audience = t.audience;
    _shareEnabled = t.shareEnabled;
    _seedEnabled = t.seedEnabled;
    _bwLimit = t.bandwidthLimitKbps.clamp(0, 8192).toDouble();
    widget.categoriesController.text = t.allowedCategories.join(', ');
    widget.didsController.text = t.allowedContactDids.join(', ');
    _selectedSessionDids.addAll(t.allowedSessionDids);
  }

  wb.OntologyTorrentPolicy _buildPolicy() {
    final cats = widget.categoriesController.text
        .split(RegExp(r'[,\s]+'))
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    final dids = widget.didsController.text
        .split(RegExp(r'[,\s]+'))
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    return wb.OntologyTorrentPolicy(
      seedEnabled: _seedEnabled,
      shareEnabled: _shareEnabled,
      audience: _audience,
      allowedCategories: cats,
      allowedContactDids: dids,
      allowedSessionDids: _selectedSessionDids.toList(),
      bandwidthLimitKbps: _bwLimit.round(),
      maxUploadMbPerDay: widget.entry.torrent.maxUploadMbPerDay,
    );
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SelectableText(
          widget.entry.magnetUri,
          style: const TextStyle(fontSize: 11, fontFamily: 'monospace'),
        ),
        const SizedBox(height: 8),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: const Text('Enable sharing'),
          value: _shareEnabled,
          onChanged: (v) => setState(() => _shareEnabled = v),
        ),
        DropdownButtonFormField<String>(
          initialValue: _audience,
          decoration: const InputDecoration(
            labelText: 'Share with',
            border: OutlineInputBorder(),
            isDense: true,
          ),
          items: const [
            DropdownMenuItem(value: 'private', child: Text('Only me')),
            DropdownMenuItem(value: 'addressbook_all', child: Text('All address book contacts')),
            DropdownMenuItem(value: 'addressbook_category', child: Text('Contacts by category')),
            DropdownMenuItem(value: 'specific_dids', child: Text('Specific DIDs')),
            DropdownMenuItem(value: 'chat_sessions', child: Text('Chat sessions / groups')),
            DropdownMenuItem(value: 'public_seed', child: Text('Public WebTorrent seed')),
          ],
          onChanged: (v) => setState(() => _audience = v ?? _audience),
        ),
        if (_audience == 'addressbook_category') ...[
          const SizedBox(height: 8),
          TextField(
            controller: widget.categoriesController,
            decoration: InputDecoration(
              labelText: 'Contact categories',
              hintText: widget.contacts
                  .expand((c) => c.categories)
                  .toSet()
                  .take(3)
                  .join(', '),
              border: const OutlineInputBorder(),
              isDense: true,
            ),
          ),
        ],
        if (_audience == 'specific_dids') ...[
          const SizedBox(height: 8),
          TextField(
            controller: widget.didsController,
            decoration: InputDecoration(
              labelText: 'Allowed DIDs',
              hintText: widget.contacts.map((c) => c.did).take(2).join(', '),
              border: const OutlineInputBorder(),
              isDense: true,
            ),
            maxLines: 2,
          ),
        ],
        if (_audience == 'chat_sessions') ...[
          const SizedBox(height: 8),
          Text(
            'Select chats or groups (each has a stable session DID)',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 4),
          if (widget.sessionTargets.isEmpty)
            const Text('No chat sessions yet.')
          else
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 200),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: widget.sessionTargets.length,
                itemBuilder: (context, i) {
                  final s = widget.sessionTargets[i];
                  final checked = _selectedSessionDids.contains(s.sessionDid);
                  final kindLabel = s.sessionKind == 'group' ? 'group' : 'solo';
                  return CheckboxListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    value: checked,
                    onChanged: (v) {
                      setState(() {
                        if (v == true) {
                          _selectedSessionDids.add(s.sessionDid);
                        } else {
                          _selectedSessionDids.remove(s.sessionDid);
                        }
                      });
                    },
                    title: Text(s.title, maxLines: 1, overflow: TextOverflow.ellipsis),
                    subtitle: Text(
                      '$kindLabel · ${s.participantCount} participants\n${s.sessionDid}',
                      style: const TextStyle(fontSize: 10, fontFamily: 'monospace'),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  );
                },
              ),
            ),
        ],
        const SizedBox(height: 8),
        Text('Per-ontology upload cap: ${_bwLimit.round() == 0 ? "unlimited" : "${_bwLimit.round()} KiB/s"}'),
        Slider(
          value: _bwLimit,
          min: 0,
          max: 4096,
          divisions: 32,
          onChanged: (v) => setState(() => _bwLimit = v),
        ),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: const Text('Seed via WebTorrent'),
          subtitle: const Text(
            'Seeds via Qualia daemon HTTP webseed (magnet ws= link)',
          ),
          value: _seedEnabled,
          onChanged: (v) => setState(() => _seedEnabled = v),
        ),
        Row(
          children: [
            Expanded(
              child: OutlinedButton(
                onPressed: () => widget.onSave(widget.entry, _buildPolicy()),
                child: const Text('Save policy'),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: FilledButton(
                onPressed: _seedEnabled
                    ? () => widget.onToggleSeed(widget.entry, !widget.entry.seedActive)
                    : null,
                child: Text(widget.entry.seedActive ? 'Stop seed' : 'Start seed'),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
