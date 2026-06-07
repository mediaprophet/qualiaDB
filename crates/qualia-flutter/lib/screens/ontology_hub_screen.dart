import 'package:flutter/material.dart';

import '../src/rust/api/qualia_api.dart';
import '../src/rust/api/qualia_api_extras.dart' as api_extras;
import '../src/rust/api/resource_catalog.dart' as catalog;

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

  String _searchQuery = '';
  String? _selectedDomain;
  String _sortBy = 'size';

  @override
  void initState() {
    super.initState();
    _loadFromRustResourceCatalog();
  }

  String _normalizeKey(String value) {
    return value.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]+'), '');
  }

  bool _matchesInstalledArtifact(String ontologyId, String artifactName) {
    final ontologyKey = _normalizeKey(ontologyId);
    final artifactKey = _normalizeKey(artifactName);
    return artifactKey.contains(ontologyKey);
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
          content: Text('Imported ${toImport.length} ontologies via Rust'),
        ),
      );
    }
    _selectedIds.clear();
    _applyFilters();
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
    try {
      if (ontology.downloadUrl != null && ontology.downloadUrl!.isNotEmpty) {
        await downloadAndVectorize(
          url: ontology.downloadUrl!,
          filename: '${ontology.id}.${ontology.format}',
          itemId: ontology.id,
        );
      } else {
        await catalog.importOntology(id: ontology.id);
      }
      if (!mounted) return;
      await _loadFromRustResourceCatalog();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('${ontology.name} imported')),
      );
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Import failed: $e')),
        );
      }
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
        title: const Text('Ontology Hub'),
        actions: [
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
        DropdownMenuItem(value: null, child: Text('All')),
        DropdownMenuItem(value: 'provenance', child: Text('Provenance')),
        DropdownMenuItem(value: 'policy', child: Text('Policy')),
        DropdownMenuItem(value: 'health', child: Text('Health')),
        DropdownMenuItem(value: 'validation', child: Text('Validation')),
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
        return Card(
          child: ListTile(
            leading: Checkbox(
              value: selected,
              onChanged: (_) => _toggleSelection(ontology.id),
            ),
            title: Text('${ontology.name} (${ontology.acronym ?? ontology.id})'),
            subtitle: Text('${ontology.domain} | ~${ontology.sizeMb} MB'),
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
                ontology.isDownloaded
                    ? const Chip(
                        label: Text('Imported'),
                        backgroundColor: Colors.green,
                      )
                    : ElevatedButton(
                        onPressed: () => _showDetails(ontology),
                        child: const Text('Import'),
                      ),
              ],
            ),
            onTap: () => _showDetails(ontology),
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
                        onChanged: (_) => _toggleSelection(ontology.id),
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
                    '${ontology.domain} | ~${ontology.sizeMb} MB',
                    style: const TextStyle(fontSize: 12),
                  ),
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
                        onPressed: () => _showDetails(ontology),
                        child: const Text('Import'),
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
