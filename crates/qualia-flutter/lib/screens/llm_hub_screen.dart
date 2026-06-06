import 'package:flutter/material.dart';

/// Enhanced LLM Hub Screen with Resource Catalog integration
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

  @override
  void initState() {
    super.initState();
    _loadFromResourceCatalog();
  }

  /// Loads models from the Resource Catalog (via Rust bridge in the future)
  Future<void> _loadFromResourceCatalog() async {
    setState(() => _isLoading = true);

    // TODO: Replace with actual flutter_rust_bridge call
    // Example: final models = await RustApi.loadLlmResources();
    await Future.delayed(const Duration(milliseconds: 350));

    _models = [
      LLMModel(
        id: 'phi-3-mini-4k-instruct-q4km',
        name: 'Phi-3 Mini 4K Instruct (Q4_K_M)',
        provider: 'Microsoft / Unsloth',
        sizeMb: 2400,
        quantization: 'Q4_K_M',
        license: 'MIT',
        tags: ['general', 'reasoning', 'edge'],
        recommendedFor: ['edge', 'rag'],
        isDownloaded: false,
      ),
      LLMModel(
        id: 'gemma-2-2b-it-q4km',
        name: 'Gemma 2 2B Instruct (Q4_K_M)',
        provider: 'Google / Unsloth',
        sizeMb: 1600,
        quantization: 'Q4_K_M',
        license: 'Gemma Terms',
        tags: ['general', 'multilingual', 'edge'],
        recommendedFor: ['edge'],
        isDownloaded: true,
      ),
      LLMModel(
        id: 'qwen2.5-1.5b-instruct-q4km',
        name: 'Qwen2.5 1.5B Instruct (Q4_K_M)',
        provider: 'Alibaba / Unsloth',
        sizeMb: 1100,
        quantization: 'Q4_K_M',
        license: 'Apache-2.0',
        tags: ['coding', 'reasoning', 'edge'],
        recommendedFor: ['edge', 'coding'],
        isDownloaded: false,
      ),
      LLMModel(
        id: 'mistral-7b-instruct-v0.3-q4km',
        name: 'Mistral 7B Instruct v0.3 (Q4_K_M)',
        provider: 'Mistral AI',
        sizeMb: 4100,
        quantization: 'Q4_K_M',
        license: 'Apache-2.0',
        tags: ['general', 'reasoning'],
        recommendedFor: ['balanced'],
        isDownloaded: false,
      ),
    ];

    _applyFilters();
    setState(() => _isLoading = false);
  }

  void _applyFilters() {
    _filteredModels = _models.where((model) {
      final matchesSearch =
          model.name.toLowerCase().contains(_searchQuery.toLowerCase()) ||
          model.provider.toLowerCase().contains(_searchQuery.toLowerCase());

      final matchesTag = _selectedTag == null || model.tags.contains(_selectedTag);
      final matchesQuant =
          _selectedQuantization == null || model.quantization == _selectedQuantization;

      return matchesSearch && matchesTag && matchesQuant;
    }).toList();

    setState(() {});
  }

  void _toggleViewMode() {
    setState(() {
      _isGridView = !_isGridView;
    });
  }

  void _toggleSelection(String id) {
    setState(() {
      if (_selectedIds.contains(id)) {
        _selectedIds.remove(id);
      } else {
        _selectedIds.add(id);
      }
    });
  }

  Future<void> _downloadSelected() async {
    final selectedModels =
        _models.where((m) => _selectedIds.contains(m.id)).toList();

    if (selectedModels.isEmpty) return;

    for (final model in selectedModels) {
      if (!model.isDownloaded) {
        // TODO: Call Rust download function
        await Future.delayed(const Duration(milliseconds: 600));
        setState(() {
          model.isDownloaded = true;
        });
      }
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Downloaded ${selectedModels.length} models')),
    );

    _selectedIds.clear();
    _applyFilters();
  }

  void _showModelDetails(LLMModel model) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => LLMModelDetailSheet(
        model: model,
        onDownload: () => _downloadModel(model),
      ),
    );
  }

  Future<void> _downloadModel(LLMModel model) async {
    Navigator.pop(context); // close detail sheet

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Downloading ${model.name}...')),
    );

    // TODO: Replace with real Rust call via flutter_rust_bridge
    await Future.delayed(const Duration(seconds: 2));

    setState(() {
      model.isDownloaded = true;
    });

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
            tooltip: _isGridView ? 'List View' : 'Grid View',
          ),
          if (_selectedIds.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.download),
              onPressed: _downloadSelected,
              tooltip: 'Download Selected',
            ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadFromResourceCatalog,
          ),
        ],
      ),
      body: Column(
        children: [
          // Search + Filters
          Padding(
            padding: const EdgeInsets.all(16.0),
            child: Column(
              children: [
                TextField(
                  decoration: const InputDecoration(
                    hintText: 'Search models...',
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
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        decoration: const InputDecoration(labelText: 'Filter by Tag'),
                        value: _selectedTag,
                        items: const [
                          DropdownMenuItem(value: null, child: Text('All Tags')),
                          DropdownMenuItem(value: 'edge', child: Text('Edge Friendly')),
                          DropdownMenuItem(value: 'reasoning', child: Text('Reasoning')),
                          DropdownMenuItem(value: 'coding', child: Text('Coding')),
                        ],
                        onChanged: (value) {
                          _selectedTag = value;
                          _applyFilters();
                        },
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        decoration: const InputDecoration(labelText: 'Quantization'),
                        value: _selectedQuantization,
                        items: const [
                          DropdownMenuItem(value: null, child: Text('All')),
                          DropdownMenuItem(value: 'Q4_K_M', child: Text('Q4_K_M')),
                        ],
                        onChanged: (value) {
                          _selectedQuantization = value;
                          _applyFilters();
                        },
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),

          // Content
          Expanded(
            child: _isLoading
                ? const Center(child: CircularProgressIndicator())
                : _filteredModels.isEmpty
                    ? const Center(child: Text('No models match your filters'))
                    : _isGridView
                        ? _buildGridView()
                        : _buildListView(),
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

  Widget _buildListView() {
    return ListView.builder(
      itemCount: _filteredModels.length,
      itemBuilder: (context, index) {
        final model = _filteredModels[index];
        final isSelected = _selectedIds.contains(model.id);

        return Card(
          margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
          child: ListTile(
            leading: Checkbox(
              value: isSelected,
              onChanged: (_) => _toggleSelection(model.id),
            ),
            title: Text(model.name),
            subtitle: Text('${model.provider} • ${model.sizeMb} MB • ${model.quantization}'),
            trailing: model.isDownloaded
                ? const Chip(
                    label: Text('Downloaded'),
                    backgroundColor: Colors.green,
                    labelStyle: TextStyle(color: Colors.white),
                  )
                : ElevatedButton(
                    onPressed: () => _downloadModel(model),
                    child: const Text('Download'),
                  ),
            onTap: () => _showModelDetails(model),
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
      itemCount: _filteredModels.length,
      itemBuilder: (context, index) {
        final model = _filteredModels[index];
        final isSelected = _selectedIds.contains(model.id);

        return Card(
          elevation: 2,
          child: InkWell(
            onTap: () => _showModelDetails(model),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Checkbox(
                        value: isSelected,
                        onChanged: (_) => _toggleSelection(model.id),
                      ),
                      if (model.isDownloaded)
                        const Chip(
                          label: Text('Downloaded', style: TextStyle(fontSize: 10)),
                          backgroundColor: Colors.green,
                          labelStyle: TextStyle(color: Colors.white, fontSize: 10),
                        ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(model.name, style: const TextStyle(fontWeight: FontWeight.bold)),
                  const SizedBox(height: 4),
                  Text(model.provider, style: const TextStyle(fontSize: 12, color: Colors.grey)),
                  const Spacer(),
                  Text('${model.sizeMb} MB • ${model.quantization}', style: const TextStyle(fontSize: 12)),
                  const SizedBox(height: 8),
                  if (!model.isDownloaded)
                    SizedBox(
                      width: double.infinity,
                      child: ElevatedButton(
                        onPressed: () => _downloadModel(model),
                        child: const Text('Download'),
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

/// Model class (will eventually come from Rust via flutter_rust_bridge)
class LLMModel {
  final String id;
  final String name;
  final String provider;
  final int sizeMb;
  final String quantization;
  final String license;
  final List<String> tags;
  final List<String> recommendedFor;
  bool isDownloaded;

  LLMModel({
    required this.id,
    required this.name,
    required this.provider,
    required this.sizeMb,
    required this.quantization,
    required this.license,
    required this.tags,
    required this.recommendedFor,
    this.isDownloaded = false,
  });
}

/// Improved Model Detail Sheet
class LLMModelDetailSheet extends StatelessWidget {
  final LLMModel model;
  final VoidCallback onDownload;

  const LLMModelDetailSheet({
    super.key,
    required this.model,
    required this.onDownload,
  });

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.65,
      builder: (context, scrollController) {
        return SingleChildScrollView(
          controller: scrollController,
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(model.name, style: Theme.of(context).textTheme.headlineSmall),
              const SizedBox(height: 4),
              Text(model.provider, style: const TextStyle(color: Colors.grey)),
              const SizedBox(height: 20),

              _buildInfoRow('Size', '${model.sizeMb} MB'),
              _buildInfoRow('Quantization', model.quantization),
              _buildInfoRow('License', model.license),
              const SizedBox(height: 16),

              const Text('Recommended For:', style: TextStyle(fontWeight: FontWeight.bold)),
              const SizedBox(height: 6),
              Wrap(
                spacing: 8,
                children: model.recommendedFor
                    .map((use) => Chip(label: Text(use)))
                    .toList(),
              ),
              const SizedBox(height: 16),

              const Text('Tags:', style: TextStyle(fontWeight: FontWeight.bold)),
              const SizedBox(height: 6),
              Wrap(
                spacing: 8,
                children: model.tags.map((tag) => Chip(label: Text(tag))).toList(),
              ),
              const SizedBox(height: 32),

              if (!model.isDownloaded)
                SizedBox(
                  width: double.infinity,
                  height: 50,
                  child: ElevatedButton.icon(
                    onPressed: onDownload,
                    icon: const Icon(Icons.download),
                    label: const Text('Download Model'),
                  ),
                )
              else
                const Center(
                  child: Chip(
                    label: Text('Already Downloaded'),
                    backgroundColor: Colors.green,
                    labelStyle: TextStyle(color: Colors.white),
                  ),
                ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildInfoRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          SizedBox(width: 120, child: Text(label, style: const TextStyle(color: Colors.grey))),
          Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}