import 'dart:async';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../main.dart' show activeModelPathProvider;
import '../src/rust/api/qualia_api.dart' as api;
import '../src/rust/api/qualia_api_extras.dart' as api_extras;
import '../src/rust/api/resource_catalog.dart' as catalog;

class LLMHubScreen extends ConsumerStatefulWidget {
  const LLMHubScreen({super.key});

  @override
  ConsumerState<LLMHubScreen> createState() => _LLMHubScreenState();
}

class _LLMHubScreenState extends ConsumerState<LLMHubScreen> {
  List<LLMModel> _models = [];
  List<LLMModel> _filteredModels = [];
  Set<String> _selectedIds = {};
  bool _isLoading = true;
  bool _isGridView = false;
  Map<String, api.ProgressPayload> _downloads = {};
  Timer? _downloadPollTimer;

  String _searchQuery = '';
  String? _selectedTag;
  String? _selectedQuantization;

  @override
  void initState() {
    super.initState();
    _loadFromRustResourceCatalog();
  }

  @override
  void dispose() {
    _downloadPollTimer?.cancel();
    super.dispose();
  }

  Future<void> _loadFromRustResourceCatalog() async {
    setState(() => _isLoading = true);
    try {
      final resources = await catalog.loadLlmResources();
      final local = await api.discoverModels();
      final activeModel = await api.getActiveModel();
      if (activeModel != null && activeModel.isNotEmpty) {
        ref.read(activeModelPathProvider.notifier).state = activeModel;
      }

      final localNames = local.map((m) => m.name.toLowerCase()).toSet();

      _models = resources.map((resource) {
        final filename = '${resource.id}.gguf';
        final downloaded = localNames.any(
          (name) =>
              name.contains(resource.id.toLowerCase()) ||
              name == filename.toLowerCase(),
        );
        return LLMModel(
          id: resource.id,
          name: resource.name,
          provider: resource.provider ?? 'Unknown',
          sizeMb: resource.sizeMb ?? 0,
          quantization: resource.quantization ?? 'unknown',
          license: resource.license ?? 'Unknown',
          tags: resource.tags ?? [],
          recommendedFor: resource.recommendedFor ?? [],
          downloadUrl: resource.downloadUrl,
          isDownloaded: downloaded,
          isEdgeRecommended: (resource.tags ?? []).contains('edge'),
        );
      }).toList();

      _applyFilters();
    } catch (e) {
      debugPrint('Failed to load from Rust: $e');
    }

    if (mounted) {
      setState(() => _isLoading = false);
    }
  }

  void _applyFilters() {
    _filteredModels = _models.where((model) {
      final matchSearch =
          model.name.toLowerCase().contains(_searchQuery.toLowerCase());
      final matchTag = _selectedTag == null || model.tags.contains(_selectedTag);
      final matchQuant = _selectedQuantization == null ||
          model.quantization == _selectedQuantization;
      return matchSearch && matchTag && matchQuant;
    }).toList();
    _filteredModels.sort((a, b) => a.sizeMb.compareTo(b.sizeMb));
    setState(() {});
  }

  void _toggleSelection(String id) {
    setState(() {
      _selectedIds.contains(id) ? _selectedIds.remove(id) : _selectedIds.add(id);
    });
  }

  bool _isActiveModel(LLMModel model, String activeModelPath) {
    return activeModelPath.toLowerCase().endsWith('${model.id}.gguf');
  }

  Future<void> _downloadModel(LLMModel model) async {
    final url = model.downloadUrl;
    if (url == null || url.isEmpty) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('No download URL for this model')),
        );
      }
      return;
    }

    final filename = '${model.id}.gguf';
    try {
      api.downloadModel(url: url, filename: filename, modelId: model.id);
      _startDownloadPolling();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Downloading ${model.name}...')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Download failed: $e')),
        );
      }
    }
  }

  void _startDownloadPolling() {
    _downloadPollTimer?.cancel();
    _downloadPollTimer = Timer.periodic(
      const Duration(milliseconds: 500),
      (_) => _refreshDownloads(),
    );
    _refreshDownloads();
  }

  Future<void> _refreshDownloads() async {
    if (!mounted) return;
    try {
      final active = await api.getActiveDownloads();
      setState(() {
        _downloads = {for (final download in active) download.id: download};
      });
      if (active.isEmpty ||
          active.every(
            (download) =>
                download.status == 'complete' ||
                download.status == 'cancelled' ||
                download.status == 'error',
          )) {
        _downloadPollTimer?.cancel();
        _downloadPollTimer = null;
        if (active.any((download) => download.status == 'complete')) {
          await _loadFromRustResourceCatalog();
        }
      }
    } catch (_) {}
  }

  Future<void> _cancelDownload(String id) async {
    try {
      await api.cancelDownload(id: id);
      await _refreshDownloads();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Cancel failed: $e')),
        );
      }
    }
  }

  Future<void> _setActiveModel(LLMModel model, {bool pop = false}) async {
    if (pop) Navigator.pop(context);
    final modelName = '${model.id}.gguf';
    try {
      await api.setActiveModel(modelName: modelName);
      ref.read(activeModelPathProvider.notifier).state = modelName;
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${model.name} is now active')),
        );
        setState(() {});
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not activate model: $e')),
        );
      }
    }
  }

  Future<void> _removeModel(LLMModel model, {bool pop = false}) async {
    if (pop) Navigator.pop(context);
    try {
      final message = await api_extras.removeInstalledModel(modelId: model.id);
      if (!mounted) return;
      await _loadFromRustResourceCatalog();
      final activeModel = ref.read(activeModelPathProvider);
      if (_isActiveModel(model, activeModel)) {
        ref.read(activeModelPathProvider.notifier).state = '';
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(message)),
      );
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Remove failed: $e')),
        );
      }
    }
  }

  String? _modelNameForDownload(String id) {
    for (final model in _models) {
      if (model.id == id) return model.name;
    }
    return id;
  }

  Widget _buildDownloadBanner(api.ProgressPayload download) {
    final name = _modelNameForDownload(download.id);
    final inProgress = download.status != 'complete' &&
        download.status != 'cancelled' &&
        download.status != 'error';
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    '$name - ${download.progress.toStringAsFixed(0)}% (${download.status})',
                    style: const TextStyle(fontSize: 12),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (inProgress)
                  TextButton(
                    onPressed: () => _cancelDownload(download.id),
                    child: const Text('Cancel'),
                  ),
              ],
            ),
            const SizedBox(height: 4),
            LinearProgressIndicator(
              value: download.totalBytes > BigInt.zero
                  ? download.progress / 100
                  : null,
            ),
            if (download.speedKbps > 0)
              Text(
                '${download.speedKbps.toStringAsFixed(0)} KB/s',
                style: const TextStyle(fontSize: 11, color: Colors.grey),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _downloadSelected() async {
    for (final id in _selectedIds.toList()) {
      final model = _models.firstWhere((entry) => entry.id == id);
      await _downloadModel(model);
    }
    setState(() => _selectedIds.clear());
  }

  Future<void> _browseLocalGguf() async {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['gguf'],
      dialogTitle: 'Select a GGUF model file',
    );
    if (result?.files.single.path != null) {
      final path = result!.files.single.path!;
      ref.read(activeModelPathProvider.notifier).state = path;
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Active model set: ${result.files.single.name}')),
        );
      }
    }
  }

  void _showDetails(LLMModel model) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => LLMModelDetailSheet(
        model: model,
        onDownload: () => _downloadModel(model),
        onUse: model.isDownloaded ? () => _setActiveModel(model, pop: true) : null,
        onRemove: model.isDownloaded ? () => _removeModel(model, pop: true) : null,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final activeModel = ref.watch(activeModelPathProvider);
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('LLM Hub'),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.folder_open),
            label: const Text('Browse local GGUF...'),
            onPressed: _browseLocalGguf,
          ),
          IconButton(
            icon: Icon(_isGridView ? Icons.list : Icons.grid_view),
            onPressed: () => setState(() => _isGridView = !_isGridView),
          ),
          if (_selectedIds.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.download),
              onPressed: _downloadSelected,
            ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadFromRustResourceCatalog,
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : Column(
              children: [
                if (activeModel.isNotEmpty)
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 8,
                    ),
                    color: colorScheme.primary.withOpacity(0.12),
                    child: Text(
                      'Active: ${activeModel.split(RegExp(r'[\\/]')).last}',
                      style: TextStyle(
                        color: colorScheme.primary,
                        fontSize: 12,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                if (_downloads.isNotEmpty) ..._downloads.values.map(_buildDownloadBanner),
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: TextField(
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
                ),
                Expanded(
                  child: _isGridView ? _buildGridView(activeModel) : _buildListView(activeModel),
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

  Widget _buildListView(String activeModel) {
    return ListView.builder(
      itemCount: _filteredModels.length,
      itemBuilder: (context, index) {
        final model = _filteredModels[index];
        final download = _downloads[model.id];
        final isActive = _isActiveModel(model, activeModel);
        return Card(
          child: ListTile(
            leading: Checkbox(
              value: _selectedIds.contains(model.id),
              onChanged: (_) => _toggleSelection(model.id),
            ),
            title: Text(model.name),
            subtitle: Text(
              '${model.provider} | ${model.sizeMb} MB${download != null ? " | ${download.progress.toStringAsFixed(0)}%" : ""}',
            ),
            trailing: Wrap(
              spacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                if (model.isDownloaded)
                  IconButton(
                    tooltip: 'Remove local model',
                    onPressed: () => _removeModel(model),
                    icon: const Icon(Icons.delete_outline),
                  ),
                if (model.isDownloaded)
                  OutlinedButton(
                    onPressed: () => _setActiveModel(model),
                    child: Text(isActive ? 'Active' : 'Use'),
                  )
                else
                  ElevatedButton(
                    onPressed: () => _showDetails(model),
                    child: const Text('Download'),
                  ),
              ],
            ),
            onTap: () => _showDetails(model),
          ),
        );
      },
    );
  }

  Widget _buildGridView(String activeModel) {
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
        final isActive = _isActiveModel(model, activeModel);
        return Card(
          child: InkWell(
            onTap: () => _showDetails(model),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    model.name,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                  Text('${model.sizeMb} MB | ${model.quantization}'),
                  if (model.isDownloaded)
                    Padding(
                      padding: const EdgeInsets.only(top: 8),
                      child: Wrap(
                        spacing: 8,
                        children: [
                          Chip(
                            label: Text(isActive ? 'Active' : 'Local'),
                          ),
                          IconButton(
                            tooltip: 'Remove local model',
                            onPressed: () => _removeModel(model),
                            icon: const Icon(Icons.delete_outline, size: 18),
                          ),
                        ],
                      ),
                    ),
                  const Spacer(),
                  if (!model.isDownloaded)
                    ElevatedButton(
                      onPressed: () => _downloadModel(model),
                      child: const Text('Download'),
                    )
                  else
                    Row(
                      children: [
                        Expanded(
                          child: OutlinedButton(
                            onPressed: () => _setActiveModel(model),
                            child: Text(isActive ? 'Active' : 'Use'),
                          ),
                        ),
                      ],
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

class LLMModel {
  final String id;
  final String name;
  final String provider;
  final String quantization;
  final String license;
  final int sizeMb;
  final List<String> tags;
  final List<String> recommendedFor;
  final String? downloadUrl;
  bool isDownloaded;
  final bool isEdgeRecommended;

  LLMModel({
    required this.id,
    required this.name,
    required this.provider,
    required this.sizeMb,
    required this.quantization,
    required this.license,
    required this.tags,
    required this.recommendedFor,
    this.downloadUrl,
    this.isDownloaded = false,
    this.isEdgeRecommended = false,
  });
}

class LLMModelDetailSheet extends ConsumerWidget {
  final LLMModel model;
  final VoidCallback onDownload;
  final VoidCallback? onUse;
  final VoidCallback? onRemove;

  const LLMModelDetailSheet({
    super.key,
    required this.model,
    required this.onDownload,
    this.onUse,
    this.onRemove,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.55,
      builder: (context, scrollController) => SingleChildScrollView(
        controller: scrollController,
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(model.name, style: Theme.of(context).textTheme.headlineSmall),
            Text(model.provider),
            const SizedBox(height: 20),
            Text('Size: ${model.sizeMb} MB'),
            Text('Quantization: ${model.quantization}'),
            const SizedBox(height: 24),
            if (!model.isDownloaded)
              ElevatedButton.icon(
                icon: const Icon(Icons.download),
                label: const Text('Download via Rust'),
                onPressed: () {
                  onDownload();
                  Navigator.pop(context);
                },
              ),
            if (onUse != null) ...[
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.play_circle_outline),
                  label: const Text('Use This Model'),
                  onPressed: onUse,
                ),
              ),
              const SizedBox(height: 12),
            ],
            if (onRemove != null) ...[
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.delete_outline),
                  label: const Text('Remove Local Model'),
                  onPressed: onRemove,
                ),
              ),
              const SizedBox(height: 12),
            ],
            OutlinedButton.icon(
              icon: const Icon(Icons.folder_open),
              label: const Text('Use local file...'),
              onPressed: () async {
                final result = await FilePicker.platform.pickFiles(
                  type: FileType.custom,
                  allowedExtensions: ['gguf'],
                );
                if (result?.files.single.path != null) {
                  ref.read(activeModelPathProvider.notifier).state =
                      result!.files.single.path!;
                  if (context.mounted) Navigator.pop(context);
                }
              },
            ),
          ],
        ),
      ),
    );
  }
}
