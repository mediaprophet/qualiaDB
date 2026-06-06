import 'package:flutter/material.dart';

/// Comprehensive LLM Hub Screen
///
/// This screen displays downloadable LLM resources from the Resource Catalog.
/// It supports search, filtering, model details, and download actions.
class LLMHubScreen extends StatefulWidget {
  const LLMHubScreen({super.key});

  @override
  State<LLMHubScreen> createState() => _LLMHubScreenState();
}

class _LLMHubScreenState extends State<LLMHubScreen> {
  List<LLMModel> _models = [];
  List<LLMModel> _filteredModels = [];
  bool _isLoading = true;
  String _searchQuery = '';
  String? _selectedTag;
  String? _selectedQuantization;

  @override
  void initState() {
    super.initState();
    _loadModels();
  }

  Future<void> _loadModels() async {
    setState(() => _isLoading = true);

    // TODO: Replace with actual call to Rust via flutter_rust_bridge
    // For now we use mock data that mirrors resources/llms.yaml
    await Future.delayed(const Duration(milliseconds: 400));

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
    ];

    _applyFilters();
    setState(() => _isLoading = false);
  }

  void _applyFilters() {
    _filteredModels = _models.where((model) {
      final matchesSearch = model.name.toLowerCase().contains(_searchQuery.toLowerCase()) ||
          model.provider.toLowerCase().contains(_searchQuery.toLowerCase());

      final matchesTag = _selectedTag == null || model.tags.contains(_selectedTag);
      final matchesQuant = _selectedQuantization == null || model.quantization == _selectedQuantization;

      return matchesSearch && matchesTag && matchesQuant;
    }).toList();

    setState(() {});
  }

  void _onSearchChanged(String query) {
    _searchQuery = query;
    _applyFilters();
  }

  void _onTagSelected(String? tag) {
    setState(() {
      _selectedTag = tag;
    });
    _applyFilters();
  }

  void _onQuantizationSelected(String? quant) {
    setState(() {
      _selectedQuantization = quant;
    });
    _applyFilters();
  }

  void _showModelDetails(LLMModel model) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (context) => LLMModelDetailSheet(model: model),
    );
  }

  Future<void> _downloadModel(LLMModel model) async {
    // TODO: Call Rust backend to start download using Resource Catalog
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Starting download: ${model.name}')),
    );

    // Simulate download
    await Future.delayed(const Duration(seconds: 2));

    setState(() {
      model.isDownloaded = true;
    });

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Download complete: ${model.name}')),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('LLM Hub'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadModels,
          ),
        ],
      ),
      body: Column(
        children: [
          // Search and Filters
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  decoration: const InputDecoration(
                    hintText: 'Search models...',
                    prefixIcon: Icon(Icons.search),
                    border: OutlineInputBorder(),
                  ),
                  onChanged: _onSearchChanged,
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        decoration: const InputDecoration(labelText: 'Tag'),
                        value: _selectedTag,
                        items: const [
                          DropdownMenuItem(value: null, child: Text('All Tags')),
                          DropdownMenuItem(value: 'edge', child: Text('Edge')),
                          DropdownMenuItem(value: 'reasoning', child: Text('Reasoning')),
                          DropdownMenuItem(value: 'coding', child: Text('Coding')),
                        ],
                        onChanged: _onTagSelected,
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
                        onChanged: _onQuantizationSelected,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),

          // Model List
          Expanded(
            child: _isLoading
                ? const Center(child: CircularProgressIndicator())
                : _filteredModels.isEmpty
                    ? const Center(child: Text('No models found'))
                    : ListView.builder(
                        itemCount: _filteredModels.length,
                        itemBuilder: (context, index) {
                          final model = _filteredModels[index];
                          return Card(
                            margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                            child: ListTile(
                              title: Text(model.name),
                              subtitle: Text('${model.provider} • ${model.sizeMb} MB • ${model.quantization}'),
                              trailing: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  if (model.isDownloaded)
                                    const Chip(label: Text('Downloaded'), backgroundColor: Colors.green),
                                  const SizedBox(width: 8),
                                  ElevatedButton(
                                    onPressed: model.isDownloaded
                                        ? null
                                        : () => _downloadModel(model),
                                    child: const Text('Download'),
                                  ),
                                ],
                              ),
                              onTap: () => _showModelDetails(model),
                            ),
                          );
                        },
                      ),
          ),
        ],
      ),
    );
  }
}

/// Simple model class (will be replaced by generated Rust types later)
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

/// Bottom sheet for model details
class LLMModelDetailSheet extends StatelessWidget {
  final LLMModel model;

  const LLMModelDetailSheet({super.key, required this.model});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(model.name, style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 8),
          Text(model.provider),
          const SizedBox(height: 16),
          Text('Size: ${model.sizeMb} MB'),
          Text('Quantization: ${model.quantization}'),
          Text('License: ${model.license}'),
          const SizedBox(height: 16),
          const Text('Tags:'),
          Wrap(
            spacing: 8,
            children: model.tags.map((tag) => Chip(label: Text(tag))).toList(),
          ),
          const SizedBox(height: 24),
          SizedBox(
            width: double.infinity,
            child: ElevatedButton(
              onPressed: () {
                Navigator.pop(context);
                // TODO: Trigger actual download
              },
              child: const Text('Download Model'),
            ),
          ),
        ],
      ),
    );
  }
}