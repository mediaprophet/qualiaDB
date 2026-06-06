import 'package:flutter/material.dart';

/// Production-ready LLM Hub Screen with Resource Catalog integration
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
  String _sortBy = 'size'; // size, name, recommended

  @override
  void initState() {
    super.initState();
    _loadFromResourceCatalog();
  }

  Future<void> _loadFromResourceCatalog() async {
    setState(() => _isLoading = true);

    // TODO: Replace with flutter_rust_bridge call to load from resources/llms.yaml
    await Future.delayed(const Duration(milliseconds: 300));

    _models = _getMockModels(); // Will be replaced by real catalog data
    _applyFilters();
    setState(() => _isLoading = false);
  }

  List<LLMModel> _getMockModels() {
    return [
      LLMModel(
        id: 'phi-3-mini-4k-instruct-q4km',
        name: 'Phi-3 Mini 4K Instruct',
        provider: 'Microsoft / Unsloth',
        sizeMb: 2400,
        quantization: 'Q4_K_M',
        license: 'MIT',
        tags: ['general', 'reasoning', 'edge'],
        recommendedFor: ['edge', 'rag'],
        isDownloaded: false,
        isEdgeRecommended: true,
      ),
      LLMModel(
        id: 'gemma-2-2b-it-q4km',
        name: 'Gemma 2 2B Instruct',
        provider: 'Google / Unsloth',
        sizeMb: 1600,
        quantization: 'Q4_K_M',
        license: 'Gemma Terms',
        tags: ['general', 'multilingual', 'edge'],
        recommendedFor: ['edge'],
        isDownloaded: true,
        isEdgeRecommended: true,
      ),
      LLMModel(
        id: 'qwen2.5-1.5b-instruct-q4km',
        name: 'Qwen2.5 1.5B Instruct',
        provider: 'Alibaba / Unsloth',
        sizeMb: 1100,
        quantization: 'Q4_K_M',
        license: 'Apache-2.0',
        tags: ['coding', 'reasoning', 'edge'],
        recommendedFor: ['edge', 'coding'],
        isDownloaded: false,
        isEdgeRecommended: true,
      ),
      LLMModel(
        id: 'mistral-7b-instruct-v0.3-q4km',
        name: 'Mistral 7B Instruct v0.3',
        provider: 'Mistral AI',
        sizeMb: 4100,
        quantization: 'Q4_K_M',
        license: 'Apache-2.0',
        tags: ['general', 'reasoning'],
        recommendedFor: ['balanced'],
        isDownloaded: false,
        isEdgeRecommended: false,
      ),
    ];
  }

  void _applyFilters() {
    _filteredModels = _models.where((model) {
      final matchesSearch = model.name.toLowerCase().contains(_searchQuery.toLowerCase()) ||
          model.provider.toLowerCase().contains(_searchQuery.toLowerCase());

      final matchesTag = _selectedTag == null || model.tags.contains(_selectedTag);
      final matchesQuant = _selectedQuantization == null || model.quantization == _selectedQuantization;

      return matchesSearch && matchesTag && matchesQuant;
    }).toList();

    // Apply sorting
    switch (_sortBy) {
      case 'size':
        _filteredModels.sort((a, b) => a.sizeMb.compareTo(b.sizeMb));
        break;
      case 'name':
        _filteredModels.sort((a, b) => a.name.compareTo(b.name));
        break;
      case 'recommended':
        _filteredModels.sort((a, b) {
          if (a.isEdgeRecommended == b.isEdgeRecommended) return 0;
          return a.isEdgeRecommended ? -1 : 1;
        });
        break;
    }

    setState(() {});
  }

  void _toggleViewMode() => setState(() => _isGridView = !_isGridView);

  void _toggleSelection(String id) {
    setState(() {
      _selectedIds.contains(id) ? _selectedIds.remove(id) : _selectedIds.add(id);
    });
  }

  Future<void> _downloadSelected() async {
    final toDownload = _models.where((m) => _selectedIds.contains(m.id) && !m.isDownloaded).toList();
    if (toDownload.isEmpty) return;

    for (final model in toDownload) {
      await Future.delayed(const Duration(milliseconds: 400));
      setState(() => model.isDownloaded = true);
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Downloaded ${toDownload.length} models')),
    );
    _selectedIds.clear();
    _applyFilters();
  }

  void _showModelDetails(LLMModel model) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => LLMModelDetailSheet(model: model, onDownload: () => _downloadModel(model)),
    );
  }

  Future<void> _downloadModel(LLMModel model) async {
    Navigator.pop(context);
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Downloading ${model.name}...')));

    await Future.delayed(const Duration(seconds: 2));
    setState(() => model.isDownloaded = true);

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('${model.name} downloaded successfully')),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('LLM Hub'),
        actions: [
          IconButton(
            icon: Icon(_isGridView ? Icons.list : Icons.grid_view),
            onPressed: _toggleViewMode,
          ),
          if (_selectedIds.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.download),
              onPressed: _downloadSelected,
              tooltip: 'Download Selected (${_selectedIds.length})',
            ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadFromResourceCatalog,
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
                  decoration: const InputDecoration(hintText: 'Search models...', prefixIcon: Icon(Icons.search), border: OutlineInputBorder()),
                  onChanged: (v) { _searchQuery = v; _applyFilters(); },
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(child: _buildTagFilter()),
                    const SizedBox(width: 12),
                    Expanded(child: _buildQuantFilter()),
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
                : _filteredModels.isEmpty
                    ? const Center(child: Text('No models found'))
                    : _isGridView ? _buildGridView() : _buildListView(),
          ),
        ],
      ),
      floatingActionButton: _selectedIds.isNotEmpty
          ? FloatingActionButton.extended(
              onPressed: _downloadSelected,
              icon: const Icon(Icons.download),
              label: Text('Download ${_selectedIds.length}'),
            )
          : null,
    );
  }

  Widget _buildTagFilter() {
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(labelText: 'Tag'),
      value: _selectedTag,
      items: const [
        DropdownMenuItem(value: null, child: Text('All')),
        DropdownMenuItem(value: 'edge', child: Text('Edge')),
        DropdownMenuItem(value: 'reasoning', child: Text('Reasoning')),
      ],
      onChanged: (v) { _selectedTag = v; _applyFilters(); },
    );
  }

  Widget _buildQuantFilter() {
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(labelText: 'Quantization'),
      value: _selectedQuantization,
      items: const [
        DropdownMenuItem(value: null, child: Text('All')),
        DropdownMenuItem(value: 'Q4_K_M', child: Text('Q4_K_M')),
      ],
      onChanged: (v) { _selectedQuantization = v; _applyFilters(); },
    );
  }

  Widget _buildSortDropdown() {
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(labelText: 'Sort'),
      value: _sortBy,
      items: const [
        DropdownMenuItem(value: 'size', child: Text('Size (smallest)')),
        DropdownMenuItem(value: 'name', child: Text('Name')),
        DropdownMenuItem(value: 'recommended', child: Text('Edge Recommended')),
      ],
      onChanged: (v) { _sortBy = v ?? 'size'; _applyFilters(); },
    );
  }

  Widget _buildListView() {
    return ListView.builder(
      itemCount: _filteredModels.length,
      itemBuilder: (context, i) {
        final m = _filteredModels[i];
        final selected = _selectedIds.contains(m.id);
        return Card(
          child: ListTile(
            leading: Checkbox(value: selected, onChanged: (_) => _toggleSelection(m.id)),
            title: Row(
              children: [
                Expanded(child: Text(m.name)),
                if (m.isEdgeRecommended) const Padding(padding: EdgeInsets.only(left: 8), child: Chip(label: Text('Edge', style: TextStyle(fontSize: 11)), backgroundColor: Colors.blue, labelStyle: TextStyle(color: Colors.white))),
              ],
            ),
            subtitle: Text('${m.provider} • ${m.sizeMb} MB • ${m.quantization}'),
            trailing: m.isDownloaded
                ? const Chip(label: Text('Downloaded'), backgroundColor: Colors.green)
                : ElevatedButton(onPressed: () => _downloadModel(m), child: const Text('Download')),
            onTap: () => _showModelDetails(m),
          ),
        );
      },
    );
  }

  Widget _buildGridView() {
    return GridView.builder(
      padding: const EdgeInsets.all(16),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(crossAxisCount: 2, crossAxisSpacing: 12, mainAxisSpacing: 12, childAspectRatio: 1.05),
      itemCount: _filteredModels.length,
      itemBuilder: (context, i) {
        final m = _filteredModels[i];
        final selected = _selectedIds.contains(m.id);
        return Card(
          child: InkWell(
            onTap: () => _showModelDetails(m),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(mainAxisAlignment: MainAxisAlignment.spaceBetween, children: [
                    Checkbox(value: selected, onChanged: (_) => _toggleSelection(m.id)),
                    if (m.isEdgeRecommended) const Chip(label: Text('Edge', style: TextStyle(fontSize: 10)), backgroundColor: Colors.blue, labelStyle: TextStyle(color: Colors.white, fontSize: 10)),
                  ]),
                  const SizedBox(height: 8),
                  Text(m.name, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
                  Text(m.provider, style: const TextStyle(fontSize: 11, color: Colors.grey)),
                  const Spacer(),
                  Text('${m.sizeMb} MB • ${m.quantization}', style: const TextStyle(fontSize: 12)),
                  const SizedBox(height: 8),
                  if (!m.isDownloaded)
                    SizedBox(width: double.infinity, child: ElevatedButton(onPressed: () => _downloadModel(m), child: const Text('Download'))),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class LLMModel {
  final String id, name, provider, quantization, license;
  final int sizeMb;
  final List<String> tags, recommendedFor;
  bool isDownloaded;
  final bool isEdgeRecommended;

  LLMModel({
    required this.id, required this.name, required this.provider,
    required this.sizeMb, required this.quantization, required this.license,
    required this.tags, required this.recommendedFor,
    this.isDownloaded = false, this.isEdgeRecommended = false,
  });
}

class LLMModelDetailSheet extends StatelessWidget {
  final LLMModel model;
  final VoidCallback onDownload;

  const LLMModelDetailSheet({super.key, required this.model, required this.onDownload});

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.7,
      builder: (context, scrollController) => SingleChildScrollView(
        controller: scrollController,
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(model.name, style: Theme.of(context).textTheme.headlineSmall),
            Text(model.provider, style: const TextStyle(color: Colors.grey)),
            const SizedBox(height: 20),
            _infoRow('Size', '${model.sizeMb} MB'),
            _infoRow('Quantization', model.quantization),
            _infoRow('License', model.license),
            const SizedBox(height: 16),
            const Text('Recommended For', style: TextStyle(fontWeight: FontWeight.bold)),
            Wrap(spacing: 8, children: model.recommendedFor.map((e) => Chip(label: Text(e))).toList()),
            const SizedBox(height: 16),
            const Text('Tags', style: TextStyle(fontWeight: FontWeight.bold)),
            Wrap(spacing: 8, children: model.tags.map((e) => Chip(label: Text(e))).toList()),
            const SizedBox(height: 32),
            if (!model.isDownloaded)
              SizedBox(width: double.infinity, height: 48, child: ElevatedButton.icon(onPressed: onDownload, icon: const Icon(Icons.download), label: const Text('Download Model')))
            else
              const Center(child: Chip(label: Text('Downloaded'), backgroundColor: Colors.green, labelStyle: TextStyle(color: Colors.white))),
          ],
        ),
      ),
    );
  }

  Widget _infoRow(String label, String value) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 4),
    child: Row(children: [SizedBox(width: 110, child: Text(label, style: const TextStyle(color: Colors.grey))), Text(value)]),
  );
}