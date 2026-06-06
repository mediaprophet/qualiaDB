import 'package:flutter/material.dart';
// TODO: Import generated bridge when flutter_rust_bridge codegen is run
// import 'package:qualia_flutter/bridge_generated.dart';

class LLMHubScreen extends StatefulWidget {
  const LLMHubScreen({super.key});

  @override
  State<LLMHubScreen> createState() => _LLMHubScreenState();
}

class _LLMHubScreenState extends State<LLMHubScreen> {
  List<LLMModel> _models = [];
  List<LLMModel> _filteredModels = [];
  Set<String> _selectedIds = {};
  bool _isLoading = true;
  bool _isGridView = false;

  String _searchQuery = '';
  String? _selectedTag;
  String? _selectedQuantization;
  String _sortBy = 'size';

  @override
  void initState() {
    super.initState();
    _loadFromRustResourceCatalog();
  }

  /// Loads LLM resources from the Rust Resource Catalog layer
  Future<void> _loadFromRustResourceCatalog() async {
    setState(() => _isLoading = true);

    try {
      // When flutter_rust_bridge is set up, replace with:
      // final resources = await api.loadLlmResources();
      // For now we simulate the data coming from Rust
      final resources = await _simulateRustLoadLlmResources();

      _models = resources.map((r) => LLMModel(
        id: r.id,
        name: r.name,
        provider: r.provider ?? 'Unknown',
        sizeMb: r.sizeMb ?? 0,
        quantization: r.quantization ?? 'unknown',
        license: r.license ?? 'Unknown',
        tags: r.tags ?? [],
        recommendedFor: r.recommendedFor ?? [],
        isDownloaded: false,
        isEdgeRecommended: (r.tags ?? []).contains('edge'),
      )).toList();

      _applyFilters();
    } catch (e) {
      debugPrint('Failed to load from Rust: $e');
    }

    setState(() => _isLoading = false);
  }

  // Temporary simulation until flutter_rust_bridge is fully wired
  Future<List<dynamic>> _simulateRustLoadLlmResources() async {
    await Future.delayed(const Duration(milliseconds: 250));
    return [
      {'id': 'phi-3-mini-4k-instruct-q4km', 'name': 'Phi-3 Mini 4K Instruct', 'provider': 'Microsoft', 'sizeMb': 2400, 'quantization': 'Q4_K_M', 'license': 'MIT', 'tags': ['general', 'reasoning', 'edge'], 'recommendedFor': ['edge', 'rag']},
      {'id': 'gemma-2-2b-it-q4km', 'name': 'Gemma 2 2B Instruct', 'provider': 'Google', 'sizeMb': 1600, 'quantization': 'Q4_K_M', 'license': 'Gemma', 'tags': ['general', 'edge'], 'recommendedFor': ['edge']},
    ];
  }

  void _applyFilters() {
    _filteredModels = _models.where((m) {
      final matchSearch = m.name.toLowerCase().contains(_searchQuery.toLowerCase());
      final matchTag = _selectedTag == null || m.tags.contains(_selectedTag);
      final matchQuant = _selectedQuantization == null || m.quantization == _selectedQuantization;
      return matchSearch && matchTag && matchQuant;
    }).toList();

    // Sorting
    switch (_sortBy) {
      case 'size':
        _filteredModels.sort((a, b) => a.sizeMb.compareTo(b.sizeMb));
        break;
      case 'name':
        _filteredModels.sort((a, b) => a.name.compareTo(b.name));
        break;
      case 'recommended':
        _filteredModels.sort((a, b) => b.isEdgeRecommended ? 1 : -1);
        break;
    }
    setState(() {});
  }

  void _toggleSelection(String id) {
    setState(() => _selectedIds.contains(id) ? _selectedIds.remove(id) : _selectedIds.add(id));
  }

  Future<void> _downloadSelected() async {
    // TODO: Call Rust download function for each selected model
    for (final id in _selectedIds) {
      // await api.downloadLlm(id);
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Downloading ${_selectedIds.length} models via Rust...')),
    );
    _selectedIds.clear();
  }

  void _showDetails(LLMModel model) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => LLMModelDetailSheet(model: model),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('LLM Hub'),
        actions: [
          IconButton(icon: Icon(_isGridView ? Icons.list : Icons.grid_view), onPressed: () => setState(() => _isGridView = !_isGridView)),
          if (_selectedIds.isNotEmpty)
            IconButton(icon: const Icon(Icons.download), onPressed: _downloadSelected),
          IconButton(icon: const Icon(Icons.refresh), onPressed: _loadFromRustResourceCatalog),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : Column(
              children: [
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    children: [
                      TextField(
                        decoration: const InputDecoration(hintText: 'Search...', prefixIcon: Icon(Icons.search), border: OutlineInputBorder()),
                        onChanged: (v) { _searchQuery = v; _applyFilters(); },
                      ),
                      const SizedBox(height: 12),
                      Row(children: [
                        Expanded(child: _buildFilterDropdown('Tag', _selectedTag, ['edge', 'reasoning'], (v) { _selectedTag = v; _applyFilters(); })),
                        const SizedBox(width: 12),
                        Expanded(child: _buildFilterDropdown('Quantization', _selectedQuantization, ['Q4_K_M'], (v) { _selectedQuantization = v; _applyFilters(); })),
                      ]),
                    ],
                  ),
                ),
                Expanded(child: _isGridView ? _buildGridView() : _buildListView()),
              ],
            ),
      floatingActionButton: _selectedIds.isNotEmpty
          ? FloatingActionButton.extended(onPressed: _downloadSelected, icon: const Icon(Icons.download), label: Text('Download ${_selectedIds.length}'))
          : null,
    );
  }

  Widget _buildFilterDropdown(String label, String? value, List<String> options, Function(String?) onChanged) {
    return DropdownButtonFormField<String>(
      decoration: InputDecoration(labelText: label),
      value: value,
      items: [const DropdownMenuItem(value: null, child: Text('All')), ...options.map((o) => DropdownMenuItem(value: o, child: Text(o)))],
      onChanged: onChanged,
    );
  }

  Widget _buildListView() => ListView.builder(
    itemCount: _filteredModels.length,
    itemBuilder: (context, i) {
      final m = _filteredModels[i];
      final selected = _selectedIds.contains(m.id);
      return Card(child: ListTile(
        leading: Checkbox(value: selected, onChanged: (_) => _toggleSelection(m.id)),
        title: Text(m.name),
        subtitle: Text('${m.provider} • ${m.sizeMb} MB'),
        trailing: m.isDownloaded ? const Chip(label: Text('Downloaded')) : ElevatedButton(onPressed: () => _showDetails(m), child: const Text('Download')),
        onTap: () => _showDetails(m),
      ));
    },
  );

  Widget _buildGridView() => GridView.builder(
    padding: const EdgeInsets.all(16),
    gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(crossAxisCount: 2, crossAxisSpacing: 12, mainAxisSpacing: 12, childAspectRatio: 1.1),
    itemCount: _filteredModels.length,
    itemBuilder: (context, i) {
      final m = _filteredModels[i];
      return Card(child: InkWell(onTap: () => _showDetails(m), child: Padding(padding: const EdgeInsets.all(12), child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(m.name, style: const TextStyle(fontWeight: FontWeight.bold)),
          Text('${m.sizeMb} MB • ${m.quantization}'),
          const Spacer(),
          if (!m.isDownloaded) ElevatedButton(onPressed: () => _showDetails(m), child: const Text('Download')),
        ],
      ))));
    },
  );
}

class LLMModel {
  final String id, name, provider, quantization, license;
  final int sizeMb;
  final List<String> tags, recommendedFor;
  bool isDownloaded;
  final bool isEdgeRecommended;

  LLMModel({required this.id, required this.name, required this.provider, required this.sizeMb,
    required this.quantization, required this.license, required this.tags, required this.recommendedFor,
    this.isDownloaded = false, this.isEdgeRecommended = false});
}

class LLMModelDetailSheet extends StatelessWidget {
  final LLMModel model;

  const LLMModelDetailSheet({super.key, required this.model});

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.65,
      builder: (context, scrollController) => SingleChildScrollView(
        controller: scrollController,
        padding: const EdgeInsets.all(24),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(model.name, style: Theme.of(context).textTheme.headlineSmall),
          Text(model.provider),
          const SizedBox(height: 20),
          Text('Size: ${model.sizeMb} MB'),
          Text('Quantization: ${model.quantization}'),
          Text('License: ${model.license}'),
          const SizedBox(height: 24),
          ElevatedButton(onPressed: () {}, child: const Text('Download via Rust')),
        ]),
      ),
    );
  }
}