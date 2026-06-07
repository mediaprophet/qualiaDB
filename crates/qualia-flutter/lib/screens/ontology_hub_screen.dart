import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../src/rust/api/qualia_api.dart' as api;
import '../src/rust/api/qualia_api_extras.dart' as api_extras;
import '../src/rust/api/resource_catalog.dart' as catalog;
import '../widgets/ontology_workbench_sheet.dart';

/// Ontology Hub integrated with Rust resource catalog and local install state.
class OntologyHubScreen extends StatefulWidget {
  const OntologyHubScreen({super.key});

  @override
  State<OntologyHubScreen> createState() => _OntologyHubScreenState();
}

class _OntologyHubScreenState extends State<OntologyHubScreen> {
  List<OntologyModel> _ontologies = [];
  List<OntologyModel> _filtered = [];
  Set<String> _selectedIds = {};
  bool _isLoading = true;
  bool _isGridView = false;
  String _catalogSource = 'bundled';
  Map<String, api.ProgressPayload> _imports = {};
  Timer? _importPollTimer;

  String _searchQuery = '';
  String? _selectedDomain;
  String _sortBy = 'size';

  @override
  void initState() {
    super.initState();
    _loadFromRustResourceCatalog();
    _restoreActiveImports();
  }

  @override
  void dispose() {
    _importPollTimer?.cancel();
    super.dispose();
  }

  String _normalizeKey(String value) {
    return value.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]+'), '');
  }

  bool _matchesInstalledArtifact(String ontologyId, String artifactName) {
    final ontologyKey = _normalizeKey(ontologyId);
    final artifactKey = _normalizeKey(artifactName);
    return artifactKey.contains(ontologyKey);
  }

  double _parseSizeMb(String? raw, {double fallback = 0}) {
    if (raw == null || raw.isEmpty) return fallback;
    final match = RegExp(r'([\d.]+)').firstMatch(raw);
    if (match == null) return fallback;
    final value = double.tryParse(match.group(1) ?? '') ?? fallback;
    final lower = raw.toLowerCase();
    if (lower.contains('gb')) return value * 1024;
    if (lower.contains('kb')) return value / 1024;
    return value;
  }

  String _domainForManifestEntry(Map<String, dynamic> item) {
    final id = (item['id'] as String? ?? '').toLowerCase();
    if (id.contains('geo')) return 'geography';
    if (id.contains('chebi') || id.contains('snomed')) return 'health';
    if (id.contains('wordnet')) return 'linguistics';
    if (id.contains('prov')) return 'provenance';
    if (id.contains('odrl')) return 'policy';
    if (id.contains('shacl')) return 'validation';
    if (id.contains('foaf')) return 'social';
    return 'general';
  }

  Future<void> _mergeRemoteManifest(List<String> installedArtifacts) async {
    try {
      final json = await api.fetchRemoteManifest(
        url:
            'https://raw.githubusercontent.com/mediaprophet/qualiaDB/refs/heads/main/manifests/ontologies.json',
      );
      final data = jsonDecode(json) as Map<String, dynamic>;
      final remote = data['ontologies'] as List<dynamic>? ?? [];
      if (remote.isEmpty) return;

      final existingIds = _ontologies.map((o) => o.id).toSet();
      var added = 0;
      for (final entry in remote) {
        if (entry is! Map<String, dynamic>) continue;
        final id = entry['id'] as String?;
        if (id == null || id.isEmpty || existingIds.contains(id)) continue;

        _ontologies.add(
          OntologyModel(
            id: id,
            name: entry['name'] as String? ?? id,
            acronym: id,
            domain: _domainForManifestEntry(entry),
            sizeMb: _parseSizeMb(entry['size'] as String?),
            format: (entry['type'] as String? ?? 'rdf').toLowerCase(),
            license: 'Unknown',
            downloadUrl: entry['url'] as String?,
            isDownloaded: installedArtifacts.any(
              (artifact) => _matchesInstalledArtifact(id, artifact),
            ),
          ),
        );
        existingIds.add(id);
        added++;
      }

      if (added > 0 && mounted) {
        setState(() => _catalogSource = 'bundled + remote');
      } else if (mounted) {
        setState(() => _catalogSource = 'bundled (remote synced)');
      }
    } catch (e) {
      debugPrint('Remote ontology manifest unavailable: $e');
      if (mounted) setState(() => _catalogSource = 'bundled');
    }
  }

  Future<void> _loadFromRustResourceCatalog() async {
    setState(() => _isLoading = true);

    try {
      final resources = await catalog.loadOntologyResources();
      final installedArtifacts = await api_extras.listInstalledOntologyArtifacts();

      _ontologies = resources
          .map(
            (r) => OntologyModel(
              id: r.id,
              name: r.name,
              acronym: r.acronym,
              domain: r.domain ?? 'general',
              sizeMb: (r.sizeEstimateMb ?? 0).toDouble(),
              format: r.format,
              license: r.license ?? 'Unknown',
              downloadUrl: r.downloadUrl,
              isDownloaded: installedArtifacts.any(
                (artifact) => _matchesInstalledArtifact(r.id, artifact),
              ),
            ),
          )
          .toList();

      await _mergeRemoteManifest(installedArtifacts);
      _applyFilters();
    } catch (e) {
      debugPrint('Failed to load ontologies from Rust: $e');
    }

    if (mounted) {
      setState(() => _isLoading = false);
    }
  }

  void _applyFilters() {
    _filtered = _ontologies.where((o) {
      final matchSearch =
          o.name.toLowerCase().contains(_searchQuery.toLowerCase());
      final matchDomain = _selectedDomain == null || o.domain == _selectedDomain;
      return matchSearch && matchDomain;
    }).toList();

    switch (_sortBy) {
      case 'size':
        _filtered.sort((a, b) => a.sizeMb.compareTo(b.sizeMb));
        break;
      case 'name':
        _filtered.sort((a, b) => a.name.compareTo(b.name));
        break;
    }

    setState(() {});
  }

  void _toggleSelection(String id) {
    setState(() {
      _selectedIds.contains(id) ? _selectedIds.remove(id) : _selectedIds.add(id);
    });
  }

  Future<void> _importSelected() async {
    final toImport = _ontologies
        .where((o) => _selectedIds.contains(o.id) && !o.isDownloaded)
        .toList();
    if (toImport.isEmpty) return;

    for (final ontology in toImport) {
      await _importOntology(ontology, pop: false);
    }

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Imported ${toImport.length} ontologies'),
        ),
      );
    }
    _selectedIds.clear();
    _applyFilters();
  }

  void _beginImportTracking(OntologyModel ontology) {
    setState(() {
      _imports[ontology.id] = api.ProgressPayload(
        id: ontology.id,
        progress: 0,
        downloadedBytes: BigInt.zero,
        totalBytes: BigInt.from((ontology.sizeMb * 1024 * 1024).round()),
        speedKbps: 0,
        status: 'starting',
      );
    });
    _startImportPolling();
  }

  Future<void> _restoreActiveImports() async {
    try {
      final active = await api.getActiveDownloads();
      if (!mounted || active.isEmpty) return;
      setState(() {
        for (final item in active) {
          _imports[item.id] = item;
        }
      });
      _startImportPolling();
    } catch (_) {}
  }

  bool _isImportInProgress(String id) {
    final item = _imports[id];
    if (item == null) return false;
    return item.status == 'starting' ||
        item.status == 'downloading' ||
        item.status == 'processing';
  }

  void _startImportPolling() {
    if (_importPollTimer != null) return;
    _importPollTimer = Timer.periodic(
      const Duration(milliseconds: 250),
      (_) => _refreshImports(),
    );
    _refreshImports();
  }

  void _maybeStopImportPolling() {
    final inProgress =
        _imports.values.any((item) => _isImportInProgress(item.id));
    if (!inProgress) {
      _importPollTimer?.cancel();
      _importPollTimer = null;
    }
  }

  Future<void> _refreshImports() async {
    if (!mounted) return;
    try {
      final active = await api.getActiveDownloads();
      if (!mounted) return;
      setState(() {
        for (final item in active) {
          _imports[item.id] = item;
        }
      });
      _maybeStopImportPolling();
    } catch (_) {}
  }

  Future<void> _cancelImport(String id) async {
    try {
      await api.cancelDownload(id: id);
      if (mounted) {
        setState(() => _imports.remove(id));
        _maybeStopImportPolling();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Cancel failed: $e')),
        );
      }
    }
  }

  String? _ontologyNameForImport(String id) {
    for (final ontology in _ontologies) {
      if (ontology.id == id) return ontology.name;
    }
    return id;
  }

  String _formatBytes(BigInt bytes) {
    if (bytes <= BigInt.zero) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB'];
    var size = bytes.toDouble();
    var unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit++;
    }
    return '${size.toStringAsFixed(unit == 0 ? 0 : 1)} ${units[unit]}';
  }

  double? _importProgressValue(api.ProgressPayload item) {
    if (item.totalBytes > BigInt.zero) {
      return (item.progress / 100).clamp(0.0, 1.0);
    }
    if (item.status == 'processing') return null;
    return null;
  }

  String _importStatusLabel(api.ProgressPayload item) {
    switch (item.status) {
      case 'starting':
        return 'Connecting...';
      case 'processing':
        return 'Compiling to .q42...';
      case 'downloading':
        return '${item.progress.toStringAsFixed(0)}%';
      case 'complete':
        return 'Complete';
      case 'cancelled':
        return 'Cancelled';
      case 'error':
        return 'Failed';
      default:
        return item.status;
    }
  }

  Widget _buildImportProgress(api.ProgressPayload item) {
    final transferred = _formatBytes(item.downloadedBytes);
    final total = item.totalBytes > BigInt.zero
        ? _formatBytes(item.totalBytes)
        : null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                total == null ? transferred : '$transferred / $total',
                style: const TextStyle(fontSize: 11),
              ),
            ),
            if (item.speedKbps > 0)
              Text(
                '${item.speedKbps.toStringAsFixed(0)} KB/s',
                style: const TextStyle(fontSize: 11, color: Colors.grey),
              ),
          ],
        ),
        const SizedBox(height: 4),
        LinearProgressIndicator(value: _importProgressValue(item)),
      ],
    );
  }

  Widget _buildImportBanner(api.ProgressPayload item) {
    final name = _ontologyNameForImport(item.id);
    final inProgress = _isImportInProgress(item.id);
    return Material(
      color: Theme.of(context).colorScheme.secondaryContainer.withValues(alpha: 0.35),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                if (inProgress)
                  const Padding(
                    padding: EdgeInsets.only(right: 8),
                    child: SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
                Expanded(
                  child: Text(
                    '$name — ${_importStatusLabel(item)}',
                    style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (inProgress)
                  TextButton(
                    onPressed: () => _cancelImport(item.id),
                    child: const Text('Cancel'),
                  ),
              ],
            ),
            const SizedBox(height: 6),
            _buildImportProgress(item),
          ],
        ),
      ),
    );
  }

  void _showDetails(OntologyModel ontology) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => OntologyDetailSheet(
        ontology: ontology,
        onImport: () => _importOntology(ontology),
        onRemove:
            ontology.isDownloaded ? () => _removeOntology(ontology) : null,
      ),
    );
  }

  Future<void> _importOntology(
    OntologyModel ontology, {
    bool pop = true,
  }) async {
    if (pop) Navigator.pop(context);
    if (_isImportInProgress(ontology.id)) return;

    _beginImportTracking(ontology);
    try {
      await catalog.importOntology(id: ontology.id);
      if (!mounted) return;
      setState(() => _imports.remove(ontology.id));
      await _loadFromRustResourceCatalog();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            '${ontology.name} compiled to Index/${ontology.id}.q42',
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _imports[ontology.id] = api.ProgressPayload(
          id: ontology.id,
          progress: 0,
          downloadedBytes: BigInt.zero,
          totalBytes: BigInt.from((ontology.sizeMb * 1024 * 1024).round()),
          speedKbps: 0,
          status: e.toString() == 'Cancelled' ? 'cancelled' : 'error',
        );
      });
      if (e.toString() != 'Cancelled') {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Import failed: $e')),
        );
      }
    } finally {
      _maybeStopImportPolling();
    }
  }

  Future<void> _removeOntology(
    OntologyModel ontology, {
    bool pop = true,
  }) async {
    if (pop) Navigator.pop(context);
    try {
      final message = await api_extras.removeInstalledOntology(
        ontologyId: ontology.id,
      );
      if (!mounted) return;
      await _loadFromRustResourceCatalog();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(message)),
      );
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Removal failed: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Ontology Hub'),
            if (!_isLoading)
              Text(
                '${_filtered.length} of ${_ontologies.length} ontologies · $_catalogSource',
                style: Theme.of(context).textTheme.labelSmall,
              ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.build_circle_outlined),
            tooltip: 'Ontology workbench',
            onPressed: () => showModalBottomSheet(
              context: context,
              isScrollControlled: true,
              builder: (_) => SizedBox(
                height: MediaQuery.of(context).size.height * 0.88,
                child: const OntologyWorkbenchSheet(),
              ),
            ),
          ),
          IconButton(
            icon: Icon(_isGridView ? Icons.list : Icons.grid_view),
            onPressed: () => setState(() => _isGridView = !_isGridView),
          ),
          if (_selectedIds.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.download),
              onPressed: _importSelected,
            ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadFromRustResourceCatalog,
          ),
        ],
      ),
      body: Column(
        children: [
          if (_imports.values.any(
            (item) => _isImportInProgress(item.id) || item.status == 'error',
          ))
            ..._imports.values
                .where(
                  (item) => _isImportInProgress(item.id) || item.status == 'error',
                )
                .map(_buildImportBanner),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  decoration: const InputDecoration(
                    hintText: 'Search...',
                    prefixIcon: Icon(Icons.search),
                    border: OutlineInputBorder(),
                  ),
                  onChanged: (value) {
                    _searchQuery = value;
                    _applyFilters();
                  },
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(child: _buildDomainFilter()),
                    const SizedBox(width: 12),
                    Expanded(child: _buildSortDropdown()),
                  ],
                ),
              ],
            ),
          ),
          Expanded(
            child: _isLoading
                ? const Center(child: CircularProgressIndicator())
                : _filtered.isEmpty
                    ? const Center(child: Text('No ontologies found'))
                    : _isGridView
                        ? _buildGridView()
                        : _buildListView(),
          ),
        ],
      ),
      floatingActionButton: _selectedIds.isNotEmpty
          ? FloatingActionButton.extended(
              onPressed: _importSelected,
              icon: const Icon(Icons.download),
              label: Text('Import ${_selectedIds.length}'),
            )
          : null,
    );
  }

  Widget _buildDomainFilter() {
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(labelText: 'Domain'),
      value: _selectedDomain,
      items: const [
        DropdownMenuItem(value: null, child: Text('All domains')),
        DropdownMenuItem(value: 'general', child: Text('General')),
        DropdownMenuItem(value: 'provenance', child: Text('Provenance')),
        DropdownMenuItem(value: 'policy', child: Text('Policy')),
        DropdownMenuItem(value: 'health', child: Text('Health')),
        DropdownMenuItem(value: 'validation', child: Text('Validation')),
        DropdownMenuItem(value: 'social', child: Text('Social')),
        DropdownMenuItem(value: 'geography', child: Text('Geography')),
        DropdownMenuItem(value: 'linguistics', child: Text('Linguistics')),
        DropdownMenuItem(value: 'science', child: Text('Science')),
      ],
      onChanged: (value) {
        _selectedDomain = value;
        _applyFilters();
      },
    );
  }

  Widget _buildSortDropdown() {
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(labelText: 'Sort'),
      value: _sortBy,
      items: const [
        DropdownMenuItem(value: 'size', child: Text('Size')),
        DropdownMenuItem(value: 'name', child: Text('Name')),
      ],
      onChanged: (value) {
        _sortBy = value ?? 'size';
        _applyFilters();
      },
    );
  }

  Widget _buildListView() {
    return ListView.builder(
      itemCount: _filtered.length,
      itemBuilder: (context, index) {
        final ontology = _filtered[index];
        final selected = _selectedIds.contains(ontology.id);
        final importItem = _imports[ontology.id];
        final importing = importItem != null && _isImportInProgress(ontology.id);
        return Card(
          child: Column(
            children: [
              ListTile(
                leading: Checkbox(
                  value: selected,
                  onChanged: importing ? null : (_) => _toggleSelection(ontology.id),
                ),
                title: Text('${ontology.name} (${ontology.acronym ?? ontology.id})'),
                subtitle: Text(
                  '${ontology.domain} | ~${ontology.sizeMb} MB → .q42'
                  '${importing ? " | ${_importStatusLabel(importItem!)}" : ""}',
                ),
                trailing: Wrap(
                  spacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    if (ontology.isDownloaded)
                      IconButton(
                        tooltip: 'Remove ontology',
                        onPressed: () => _removeOntology(ontology, pop: false),
                        icon: const Icon(Icons.delete_outline),
                      ),
                    if (ontology.isDownloaded)
                      const Chip(
                        label: Text('Imported'),
                        backgroundColor: Colors.green,
                      )
                    else if (importing)
                      OutlinedButton(
                        onPressed: null,
                        child: const Text('Importing...'),
                      )
                    else
                      ElevatedButton(
                        onPressed: () => _importOntology(ontology, pop: false),
                        child: const Text('Import'),
                      ),
                  ],
                ),
                onTap: () => _showDetails(ontology),
              ),
              if (importing)
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
                  child: _buildImportProgress(importItem!),
                ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildGridView() {
    return GridView.builder(
      padding: const EdgeInsets.all(16),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 2,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
        childAspectRatio: 1.1,
      ),
      itemCount: _filtered.length,
      itemBuilder: (context, index) {
        final ontology = _filtered[index];
        final importItem = _imports[ontology.id];
        final importing = importItem != null && _isImportInProgress(ontology.id);
        return Card(
          child: InkWell(
            onTap: () => _showDetails(ontology),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Checkbox(
                        value: _selectedIds.contains(ontology.id),
                        onChanged: importing ? null : (_) => _toggleSelection(ontology.id),
                      ),
                      if (ontology.isDownloaded)
                        IconButton(
                          tooltip: 'Remove ontology',
                          onPressed: () => _removeOntology(ontology, pop: false),
                          icon: const Icon(Icons.delete_outline, size: 18),
                        ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    ontology.name,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                  Text(
                    '${ontology.domain} | ~${ontology.sizeMb} MB → .q42',
                    style: const TextStyle(fontSize: 12),
                  ),
                  if (importing) ...[
                    const SizedBox(height: 8),
                    Text(
                      _importStatusLabel(importItem!),
                      style: const TextStyle(fontSize: 12),
                    ),
                    const SizedBox(height: 4),
                    _buildImportProgress(importItem),
                  ],
                  if (ontology.isDownloaded)
                    const Padding(
                      padding: EdgeInsets.only(top: 8),
                      child: Chip(
                        label: Text('Imported', style: TextStyle(fontSize: 10)),
                        backgroundColor: Colors.green,
                      ),
                    ),
                  const Spacer(),
                  if (!ontology.isDownloaded)
                    SizedBox(
                      width: double.infinity,
                      child: ElevatedButton(
                        onPressed: importing ? null : () => _importOntology(ontology, pop: false),
                        child: Text(importing ? 'Importing...' : 'Import'),
                      ),
                    ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class OntologyModel {
  final String id;
  final String name;
  final String domain;
  final String format;
  final String license;
  final String? acronym;
  final String? downloadUrl;
  final double sizeMb;
  bool isDownloaded;

  OntologyModel({
    required this.id,
    required this.name,
    required this.domain,
    required this.format,
    required this.license,
    this.acronym,
    this.downloadUrl,
    required this.sizeMb,
    this.isDownloaded = false,
  });
}

class OntologyDetailSheet extends StatelessWidget {
  final OntologyModel ontology;
  final VoidCallback onImport;
  final VoidCallback? onRemove;

  const OntologyDetailSheet({
    super.key,
    required this.ontology,
    required this.onImport,
    this.onRemove,
  });

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.6,
      builder: (context, scrollController) => SingleChildScrollView(
        controller: scrollController,
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(ontology.name, style: Theme.of(context).textTheme.headlineSmall),
            if (ontology.acronym != null) Text(ontology.acronym!),
            const SizedBox(height: 20),
            Text('Domain: ${ontology.domain}'),
            Text('Format: ${ontology.format}'),
            Text('Size: ~${ontology.sizeMb} MB'),
            Text('License: ${ontology.license}'),
            const SizedBox(height: 12),
            Text(
              'Import downloads RDF/OWL, compiles to Index/${ontology.id}.q42, '
              'then removes the raw source file to save disk space.',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 32),
            if (!ontology.isDownloaded)
              SizedBox(
                width: double.infinity,
                height: 48,
                child: ElevatedButton.icon(
                  onPressed: onImport,
                  icon: const Icon(Icons.download),
                  label: const Text('Import Ontology'),
                ),
              )
            else
              Column(
                children: [
                  const Center(
                    child: Chip(
                      label: Text('Already Imported'),
                      backgroundColor: Colors.green,
                      labelStyle: TextStyle(color: Colors.white),
                    ),
                  ),
                  if (onRemove != null) ...[
                    const SizedBox(height: 16),
                    SizedBox(
                      width: double.infinity,
                      height: 48,
                      child: OutlinedButton.icon(
                        onPressed: onRemove,
                        icon: const Icon(Icons.delete_outline),
                        label: const Text('Remove Local Artifacts'),
                      ),
                    ),
                  ],
                ],
              ),
          ],
        ),
      ),
    );
  }
}
