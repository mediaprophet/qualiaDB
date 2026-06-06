import 'package:flutter/material.dart';

/// Comprehensive Ontology Hub Screen
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
    _loadFromResourceCatalog();
  }

  Future<void> _loadFromResourceCatalog() async {
    setState(() => _isLoading = true);
    await Future.delayed(const Duration(milliseconds: 300));

    _ontologies = [
      OntologyModel(
        id: 'prov-o',
        name: 'PROV-O',
        acronym: 'PROV-O',
        domain: 'provenance',
        sizeMb: 0.2,
        format: 'owl',
        license: 'W3C',
        isDownloaded: true,
      ),
      OntologyModel(
        id: 'odrl',
        name: 'ODRL Vocabulary',
        acronym: 'ODRL',
        domain: 'policy',
        sizeMb: 0.15,
        format: 'ttl',
        license: 'W3C',
        isDownloaded: false,
      ),
      OntologyModel(
        id: 'schema-org',
        name: 'Schema.org',
        acronym: 'schema',
        domain: 'general',
        sizeMb: 5,
        format: 'rdfa',
        license: 'CC-BY-SA',
        isDownloaded: false,
      ),
      OntologyModel(
        id: 'snomedct-us',
        name: 'SNOMED CT US Edition',
        acronym: 'SNOMEDCT',
        domain: 'health',
        sizeMb: 850,
        format: 'owl',
        license: 'UMLS',
        isDownloaded: false,
      ),
      OntologyModel(
        id: 'shacl',
        name: 'SHACL',
        acronym: 'SHACL',
        domain: 'validation',
        sizeMb: 0.3,
        format: 'ttl',
        license: 'W3C',
        isDownloaded: true,
      ),
    ];

    _applyFilters();
    setState(() => _isLoading = false);
  }

  void _applyFilters() {
    _filtered = _ontologies.where((o) {
      final matchSearch = o.name.toLowerCase().contains(_searchQuery.toLowerCase()) ||
          (o.acronym?.toLowerCase().contains(_searchQuery.toLowerCase()) ?? false);
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
    final selected = _ontologies.where((o) => _selectedIds.contains(o.id) && !o.isDownloaded).toList();
    if (selected.isEmpty) return;

    for (final ont in selected) {
      await Future.delayed(const Duration(milliseconds: 500));
      setState(() => ont.isDownloaded = true);
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Imported ${selected.length} ontologies')),
    );
    _selectedIds.clear();
    _applyFilters();
  }

  void _showDetails(OntologyModel ont) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => OntologyDetailSheet(ontology: ont, onImport: () => _importOntology(ont)),
    );
  }

  Future<void> _importOntology(OntologyModel ont) async {
    Navigator.pop(context);
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Importing ${ont.name}...')));

    await Future.delayed(const Duration(seconds: 2));
    setState(() => ont.isDownloaded = true);

    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('${ont.name} imported')));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Ontology Hub'),
        actions: [
          IconButton(icon: Icon(_isGridView ? Icons.list : Icons.grid_view), onPressed: () => setState(() => _isGridView = !_isGridView)),
          if (_selectedIds.isNotEmpty)
            IconButton(icon: const Icon(Icons.download), onPressed: _importSelected),
          IconButton(icon: const Icon(Icons.refresh), onPressed: _loadFromResourceCatalog),
        ],
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  decoration: const InputDecoration(hintText: 'Search ontologies...', prefixIcon: Icon(Icons.search), border: OutlineInputBorder()),
                  onChanged: (v) { _searchQuery = v; _applyFilters(); },
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        decoration: const InputDecoration(labelText: 'Domain'),
                        value: _selectedDomain,
                        items: const [
                          DropdownMenuItem(value: null, child: Text('All Domains')),
                          DropdownMenuItem(value: 'provenance', child: Text('Provenance')),
                          DropdownMenuItem(value: 'policy', child: Text('Policy')),
                          DropdownMenuItem(value: 'health', child: Text('Health')),
                          DropdownMenuItem(value: 'validation', child: Text('Validation')),
                        ],
                        onChanged: (v) { _selectedDomain = v; _applyFilters(); },
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        decoration: const InputDecoration(labelText: 'Sort'),
                        value: _sortBy,
                        items: const [
                          DropdownMenuItem(value: 'size', child: Text('Size')),
                          DropdownMenuItem(value: 'name', child: Text('Name')),
                        ],
                        onChanged: (v) { _sortBy = v ?? 'size'; _applyFilters(); },
                      ),
                    ),
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
                    : _isGridView ? _buildGridView() : _buildListView(),
          ),
        ],
      ),
      floatingActionButton: _selectedIds.isNotEmpty
          ? FloatingActionButton.extended(onPressed: _importSelected, icon: const Icon(Icons.download), label: Text('Import ${_selectedIds.length}'))
          : null,
    );
  }

  Widget _buildListView() {
    return ListView.builder(
      itemCount: _filtered.length,
      itemBuilder: (context, i) {
        final o = _filtered[i];
        final selected = _selectedIds.contains(o.id);
        return Card(
          child: ListTile(
            leading: Checkbox(value: selected, onChanged: (_) => _toggleSelection(o.id)),
            title: Text('${o.name} (${o.acronym ?? o.id})'),
            subtitle: Text('${o.domain} • ~${o.sizeMb} MB • ${o.format}'),
            trailing: o.isDownloaded
                ? const Chip(label: Text('Imported'), backgroundColor: Colors.green)
                : ElevatedButton(onPressed: () => _importOntology(o), child: const Text('Import')),
            onTap: () => _showDetails(o),
          ),
        );
      },
    );
  }

  Widget _buildGridView() {
    return GridView.builder(
      padding: const EdgeInsets.all(16),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(crossAxisCount: 2, crossAxisSpacing: 12, mainAxisSpacing: 12, childAspectRatio: 1.1),
      itemCount: _filtered.length,
      itemBuilder: (context, i) {
        final o = _filtered[i];
        final selected = _selectedIds.contains(o.id);
        return Card(
          child: InkWell(
            onTap: () => _showDetails(o),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(mainAxisAlignment: MainAxisAlignment.spaceBetween, children: [
                    Checkbox(value: selected, onChanged: (_) => _toggleSelection(o.id)),
                    if (o.isDownloaded) const Chip(label: Text('Imported', style: TextStyle(fontSize: 10)), backgroundColor: Colors.green),
                  ]),
                  const SizedBox(height: 8),
                  Text(o.name, style: const TextStyle(fontWeight: FontWeight.bold)),
                  Text(o.acronym ?? '', style: const TextStyle(color: Colors.grey, fontSize: 12)),
                  const Spacer(),
                  Text('${o.domain} • ~${o.sizeMb} MB', style: const TextStyle(fontSize: 12)),
                  const SizedBox(height: 8),
                  if (!o.isDownloaded)
                    SizedBox(width: double.infinity, child: ElevatedButton(onPressed: () => _importOntology(o), child: const Text('Import'))),
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
  final String id, name, domain, format, license;
  final String? acronym;
  final double sizeMb;
  bool isDownloaded;

  OntologyModel({
    required this.id, required this.name, required this.domain, required this.format, required this.license,
    this.acronym, required this.sizeMb, this.isDownloaded = false,
  });
}

class OntologyDetailSheet extends StatelessWidget {
  final OntologyModel ontology;
  final VoidCallback onImport;

  const OntologyDetailSheet({super.key, required this.ontology, required this.onImport});

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
            if (ontology.acronym != null) Text(ontology.acronym!, style: const TextStyle(color: Colors.grey)),
            const SizedBox(height: 20),
            _infoRow('Domain', ontology.domain),
            _infoRow('Format', ontology.format),
            _infoRow('Size', '~${ontology.sizeMb} MB'),
            _infoRow('License', ontology.license),
            const SizedBox(height: 32),
            if (!ontology.isDownloaded)
              SizedBox(width: double.infinity, height: 48, child: ElevatedButton.icon(onPressed: onImport, icon: const Icon(Icons.download), label: const Text('Import Ontology')))
            else
              const Center(child: Chip(label: Text('Already Imported'), backgroundColor: Colors.green, labelStyle: TextStyle(color: Colors.white))),
          ],
        ),
      ),
    );
  }

  Widget _infoRow(String label, String value) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 6),
    child: Row(children: [SizedBox(width: 100, child: Text(label, style: const TextStyle(color: Colors.grey))), Text(value)]),
  );
}