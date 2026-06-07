import 'dart:async';
import 'dart:convert';
import 'dart:ui';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../main.dart' show activeModelPathProvider;
import '../src/rust/api/qualia_api.dart' as api;
import '../src/rust/api/qualia_api_extras.dart' as api_extras;
import '../src/rust/api/resource_catalog.dart' as catalog;

const _accent = Color(0xFF00F0FF);

class LLMHubScreen extends ConsumerStatefulWidget {
  const LLMHubScreen({super.key});

  @override
  ConsumerState<LLMHubScreen> createState() => _LLMHubScreenState();
}

class _LLMHubScreenState extends ConsumerState<LLMHubScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabs;
  List<LLMModel> _models = [];
  List<LLMModel> _filteredModels = [];
  catalog.ModelPreferencesFrb? _preferences;
  Set<String> _installedIds = {};
  Map<String, dynamic>? _lifecycle;
  api.HardwareStatus? _hardware;
  catalog.ResolvedModelPreferenceFrb? _nextMatch;
  Map<String, api.ProgressPayload> _downloads = {};
  Timer? _downloadPollTimer;

  bool _isLoading = true;
  bool _prefsDirty = false;
  String _searchQuery = '';
  String? _selectedTag;

  static const _taskOptions = <String, String>{
    'always': 'Whenever installed',
    'chat': 'General chat',
    'coding': 'Coding / reasoning',
    'vision': 'Vision / multimodal',
    'low_ram': 'Low RAM only',
  };

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 3, vsync: this);
    _tabs.addListener(() {
      if (mounted) setState(() {});
    });
    _refreshAll();
    _restoreActiveDownloads();
  }

  @override
  void dispose() {
    _downloadPollTimer?.cancel();
    _tabs.dispose();
    super.dispose();
  }

  Future<void> _refreshAll() async {
    setState(() => _isLoading = true);
    try {
      final resources = await catalog.loadLlmResources();
      final local = await api.discoverModels();
      final activeModel = await api.getActiveModel();
      if (activeModel != null && activeModel.isNotEmpty) {
        ref.read(activeModelPathProvider.notifier).state = activeModel;
      }

      _preferences = await catalog.getModelPreferences();
      _installedIds = (await catalog.listInstalledLlmIds()).toSet();

      final lifecycleJson = await catalog.getModelLifecycleStatus();
      _lifecycle = jsonDecode(lifecycleJson) as Map<String, dynamic>;
      _hardware = await api.getHardwareStatus();
      _nextMatch = await catalog.resolveModelPreference(task: 'chat');

      final localNames = local.map((m) => m.name.toLowerCase()).toSet();
      _models = resources.map((resource) {
        final downloaded = _installedIds.contains(resource.id) ||
            localNames.any(
              (name) =>
                  name.contains(resource.id.toLowerCase()) ||
                  name == '${resource.id}.gguf'.toLowerCase(),
            );
        return LLMModel(
          id: resource.id,
          name: resource.name,
          provider: resource.provider ?? 'Unknown',
          sizeMb: resource.sizeMb ?? 0,
          ramEstimateMb: resource.ramEstimateMb ?? resource.sizeMb ?? 0,
          quantization: resource.quantization ?? 'unknown',
          license: resource.license ?? 'Unknown',
          tags: resource.tags ?? [],
          recommendedFor: resource.recommendedFor ?? [],
          downloadUrl: resource.downloadUrl,
          notes: resource.notes,
          isDownloaded: downloaded,
          isMultimodal: resource.isMultimodal,
          architecture: resource.architecture,
          contextWindow: resource.contextWindow,
        );
      }).toList();

      for (final entry in local) {
        _ensureLocalModelEntry(fileName: entry.name, fullPath: null);
      }
      if (activeModel != null && activeModel.isNotEmpty) {
        _ensureLocalModelEntry(
          fileName: activeModel.split(RegExp(r'[\\/]')).last,
          fullPath: activeModel.contains(RegExp(r'[\\/]')) ? activeModel : null,
        );
      }

      _applyFilters();
      _prefsDirty = false;
    } catch (e) {
      debugPrint('LLM Hub load failed: $e');
    }
    if (mounted) setState(() => _isLoading = false);
  }

  void _applyFilters() {
    _filteredModels = _models.where((model) {
      final matchSearch = _searchQuery.isEmpty ||
          model.name.toLowerCase().contains(_searchQuery.toLowerCase()) ||
          model.id.toLowerCase().contains(_searchQuery.toLowerCase());
      final matchTag =
          _selectedTag == null || model.tags.contains(_selectedTag);
      return matchSearch && matchTag;
    }).toList();
    _filteredModels.sort((a, b) {
      if (a.isDownloaded != b.isDownloaded) {
        return a.isDownloaded ? -1 : 1;
      }
      return a.ramEstimateMb.compareTo(b.ramEstimateMb);
    });
  }

  void _ensureLocalModelEntry({required String fileName, String? fullPath}) {
    if (!fileName.toLowerCase().endsWith('.gguf')) return;
    final stem =
        fileName.replaceAll(RegExp(r'\.gguf$', caseSensitive: false), '');
    final id = fullPath != null ? 'local-$stem' : stem;
    final exists = _models.any(
      (m) =>
          m.id == id ||
          m.id == stem ||
          (m.localGgufPath != null &&
              fullPath != null &&
              _normalizePath(m.localGgufPath!) ==
                  _normalizePath(fullPath)),
    );
    if (exists) return;
    _models.insert(
      0,
      LLMModel(
        id: id,
        name: stem,
        provider: fullPath != null ? 'Local file' : 'Installed',
        sizeMb: 0,
        ramEstimateMb: 0,
        quantization: 'local',
        license: 'Local',
        tags: const ['local'],
        recommendedFor: const [],
        isDownloaded: true,
        localGgufPath: fullPath,
      ),
    );
    _installedIds.add(id);
  }

  String _normalizePath(String path) =>
      path.replaceAll('\\', '/').toLowerCase();

  bool _isActiveModel(LLMModel model, String activeModelPath) {
    if (activeModelPath.isEmpty) return false;
    if (model.localGgufPath != null && model.localGgufPath!.isNotEmpty) {
      return _normalizePath(activeModelPath) ==
          _normalizePath(model.localGgufPath!);
    }
    final base = activeModelPath.split(RegExp(r'[\\/]')).last.toLowerCase();
    return base == '${model.id}.gguf'.toLowerCase() ||
        activeModelPath.toLowerCase().endsWith('${model.id}.gguf');
  }

  Future<void> _savePreferences() async {
    if (_preferences == null) return;
    await catalog.saveModelPreferences(prefs: _preferences!);
    _prefsDirty = false;
    _nextMatch = await catalog.resolveModelPreference(task: 'chat');
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Load preferences saved')),
      );
      setState(() {});
    }
  }

  Future<void> _applyAutoSelect() async {
    try {
      await catalog.applyModelPreference(task: 'chat');
      final active = await api.getActiveModel();
      if (active != null) {
        ref.read(activeModelPathProvider.notifier).state = active;
      }
      await _refreshAll();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              _nextMatch != null
                  ? 'Loaded ${_nextMatch!.label}'
                  : 'Applied best matching model',
            ),
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Auto-load failed: $e')),
        );
      }
    }
  }

  void _reorderPreference(int oldIndex, int newIndex) {
    if (_preferences == null) return;
    final entries = List<catalog.ModelPreferenceEntryFrb>.from(
      _preferences!.entries,
    );
    if (newIndex > oldIndex) newIndex -= 1;
    final item = entries.removeAt(oldIndex);
    entries.insert(newIndex, item);
    for (var i = 0; i < entries.length; i++) {
      entries[i] = catalog.ModelPreferenceEntryFrb(
        modelId: entries[i].modelId,
        label: entries[i].label,
        priority: i + 1,
        when: entries[i].when,
      );
    }
    setState(() {
      _preferences = catalog.ModelPreferencesFrb(
        autoSelect: _preferences!.autoSelect,
        entries: entries,
      );
      _prefsDirty = true;
    });
  }

  void _updateEntryCondition(int index, String task) {
    if (_preferences == null) return;
    final entries = List<catalog.ModelPreferenceEntryFrb>.from(
      _preferences!.entries,
    );
    final e = entries[index];
    entries[index] = catalog.ModelPreferenceEntryFrb(
      modelId: e.modelId,
      label: e.label,
      priority: e.priority,
      when: catalog.ModelLoadConditionFrb(
        requireInstalled: e.when.requireInstalled,
        task: task,
        minRamGb: e.when.minRamGb,
        respectRamEstimate: e.when.respectRamEstimate,
        requireMultimodal: task == 'vision' || e.when.requireMultimodal,
      ),
    );
    setState(() {
      _preferences = catalog.ModelPreferencesFrb(
        autoSelect: _preferences!.autoSelect,
        entries: entries,
      );
      _prefsDirty = true;
    });
  }

  void _removePreference(int index) {
    if (_preferences == null) return;
    final entries = List<catalog.ModelPreferenceEntryFrb>.from(
      _preferences!.entries,
    )..removeAt(index);
    for (var i = 0; i < entries.length; i++) {
      entries[i] = catalog.ModelPreferenceEntryFrb(
        modelId: entries[i].modelId,
        label: entries[i].label,
        priority: i + 1,
        when: entries[i].when,
      );
    }
    setState(() {
      _preferences = catalog.ModelPreferencesFrb(
        autoSelect: _preferences!.autoSelect,
        entries: entries,
      );
      _prefsDirty = true;
    });
  }

  Future<void> _addToQueue(LLMModel model) async {
    if (_preferences == null) return;
    if (_preferences!.entries.any((e) => e.modelId == model.id)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Model already in priority queue')),
      );
      return;
    }
    final entries = List<catalog.ModelPreferenceEntryFrb>.from(
      _preferences!.entries,
    );
    final task = model.isMultimodal
        ? 'vision'
        : model.tags.contains('coding')
            ? 'coding'
            : 'chat';
    entries.add(
      catalog.ModelPreferenceEntryFrb(
        modelId: model.id,
        label: model.name,
        priority: entries.length + 1,
        when: catalog.ModelLoadConditionFrb(
          requireInstalled: true,
          task: task,
          minRamGb: null,
          respectRamEstimate: true,
          requireMultimodal: model.isMultimodal,
        ),
      ),
    );
    setState(() {
      _preferences = catalog.ModelPreferencesFrb(
        autoSelect: _preferences!.autoSelect,
        entries: entries,
      );
      _prefsDirty = true;
    });
    _tabs.animateTo(1);
  }

  Future<void> _setActiveModel(LLMModel model) async {
    try {
      if (model.localGgufPath != null && model.localGgufPath!.isNotEmpty) {
        await _activateLocalPath(model.localGgufPath!);
      } else {
        final modelName = model.id.startsWith('local-')
            ? '${model.name}.gguf'
            : '${model.id}.gguf';
        await api.setActiveModel(modelName: modelName);
        final active = await api.getActiveModel();
        ref.read(activeModelPathProvider.notifier).state =
            active ?? modelName;
      }
      await _refreshAll();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${model.name} is now active')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not activate: $e')),
        );
      }
    }
  }

  Future<void> _activateLocalPath(String path) async {
    await api.setActiveModel(modelName: path);
    ref.read(activeModelPathProvider.notifier).state = path;
    _ensureLocalModelEntry(
      fileName: path.split(RegExp(r'[\\/]')).last,
      fullPath: path,
    );
    _applyFilters();
  }

  Future<void> _browseLocalGguf() async {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['gguf'],
      dialogTitle: 'Select a GGUF model file',
    );
    final path = result?.files.single.path;
    if (path == null || path.isEmpty) return;
    try {
      await _activateLocalPath(path);
      await _refreshAll();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Active: ${result!.files.single.name}')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not activate: $e')),
        );
      }
    }
  }

  Future<void> _downloadModel(LLMModel model) async {
    if (model.downloadUrl == null || model.downloadUrl!.isEmpty) return;
    if (_isDownloadInProgress(model.id)) return;
    _beginDownloadTracking(model);
    try {
      await catalog.installCatalogLlm(id: model.id);
      if (!mounted) return;
      setState(() => _downloads.remove(model.id));
      await _refreshAll();
      if (_preferences?.autoSelect == true) {
        await _applyAutoSelect();
      }
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${model.name} installed')),
        );
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _downloads[model.id] = api.ProgressPayload(
          id: model.id,
          progress: 0,
          downloadedBytes: BigInt.zero,
          totalBytes: BigInt.from(model.sizeMb * 1024 * 1024),
          speedKbps: 0,
          status: e.toString() == 'Cancelled' ? 'cancelled' : 'error',
        );
      });
    } finally {
      _maybeStopDownloadPolling();
    }
  }

  Future<void> _removeModel(LLMModel model) async {
    try {
      final message = await api_extras.removeInstalledModel(modelId: model.id);
      if (!mounted) return;
      final activeModel = ref.read(activeModelPathProvider);
      if (_isActiveModel(model, activeModel)) {
        ref.read(activeModelPathProvider.notifier).state = '';
      }
      await _refreshAll();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Remove failed: $e')),
        );
      }
    }
  }

  void _beginDownloadTracking(LLMModel model) {
    setState(() {
      _downloads[model.id] = api.ProgressPayload(
        id: model.id,
        progress: 0,
        downloadedBytes: BigInt.zero,
        totalBytes: BigInt.from(model.sizeMb * 1024 * 1024),
        speedKbps: 0,
        status: 'starting',
      );
    });
    _startDownloadPolling();
  }

  Future<void> _restoreActiveDownloads() async {
    try {
      final active = await api.getActiveDownloads();
      if (!mounted || active.isEmpty) return;
      setState(() {
        for (final download in active) {
          _downloads[download.id] = download;
        }
      });
      _startDownloadPolling();
    } catch (_) {}
  }

  bool _isDownloadInProgress(String id) {
    final download = _downloads[id];
    if (download == null) return false;
    return download.status == 'starting' ||
        download.status == 'downloading' ||
        download.status == 'processing';
  }

  void _startDownloadPolling() {
    if (_downloadPollTimer != null) return;
    _downloadPollTimer = Timer.periodic(
      const Duration(milliseconds: 400),
      (_) => _refreshDownloads(),
    );
    _refreshDownloads();
  }

  void _maybeStopDownloadPolling() {
    final inProgress = _downloads.values.any(
      (d) =>
          d.status == 'starting' ||
          d.status == 'downloading' ||
          d.status == 'processing',
    );
    if (!inProgress) {
      _downloadPollTimer?.cancel();
      _downloadPollTimer = null;
    }
  }

  Future<void> _refreshDownloads() async {
    if (!mounted) return;
    try {
      final active = await api.getActiveDownloads();
      if (!mounted) return;
      setState(() {
        for (final download in active) {
          _downloads[download.id] = download;
        }
      });
      _maybeStopDownloadPolling();
    } catch (_) {}
  }

  Future<void> _cancelDownload(String id) async {
    await api.cancelDownload(id: id);
    if (mounted) {
      setState(() => _downloads.remove(id));
      _maybeStopDownloadPolling();
    }
  }

  @override
  Widget build(BuildContext context) {
    final activeModel = ref.watch(activeModelPathProvider);

    return Scaffold(
      backgroundColor: const Color(0xFF050508),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0A0A0F),
        title: const Text('LLM Hub', style: TextStyle(fontFamily: 'monospace')),
        bottom: TabBar(
          controller: _tabs,
          indicatorColor: _accent,
          tabs: const [
            Tab(icon: Icon(Icons.memory), text: 'Overview'),
            Tab(icon: Icon(Icons.low_priority), text: 'Load order'),
            Tab(icon: Icon(Icons.library_books), text: 'Catalog'),
          ],
        ),
        actions: [
          if (_prefsDirty)
            TextButton.icon(
              onPressed: _savePreferences,
              icon: const Icon(Icons.save, color: _accent),
              label: const Text('Save order', style: TextStyle(color: _accent)),
            ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _refreshAll,
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator(color: _accent))
          : TabBarView(
              controller: _tabs,
              children: [
                _buildOverviewTab(activeModel),
                _buildPriorityTab(),
                _buildCatalogTab(activeModel),
              ],
            ),
      floatingActionButton: _tabs.index == 2
          ? FloatingActionButton.extended(
              onPressed: _browseLocalGguf,
              icon: const Icon(Icons.folder_open),
              label: const Text('Browse GGUF'),
              backgroundColor: _accent.withValues(alpha: 0.15),
              foregroundColor: _accent,
            )
          : null,
    );
  }

  Widget _buildOverviewTab(String activeModel) {
    final activeName = activeModel.isEmpty
        ? 'None'
        : activeModel.split(RegExp(r'[\\/]')).last;
    final lifecycle = _lifecycle?['lifecycle_state']?.toString() ?? 'Unknown';
    final ramTotal = _hardware?.ramTotalGb ?? 0;
    final ramUsed = _hardware?.ramUsedGb ?? 0;
    final ramPct = ramTotal > 0 ? (ramUsed / ramTotal).clamp(0.0, 1.0) : 0.0;

    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        _glassCard(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      activeModel.isEmpty ? Icons.warning_amber : Icons.check_circle,
                      color: activeModel.isEmpty ? Colors.orange : Colors.greenAccent,
                    ),
                    const SizedBox(width: 10),
                    const Text(
                      'Active model',
                      style: TextStyle(color: Colors.grey, fontSize: 12),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  activeName,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 22,
                    fontWeight: FontWeight.bold,
                    fontFamily: 'monospace',
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  'Engine: native GGUF + wgpu/DirectML (in-process)',
                  style: TextStyle(color: Colors.grey.shade500, fontSize: 11),
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 8,
                  children: [
                    Chip(
                      label: Text('Lifecycle: $lifecycle'),
                      visualDensity: VisualDensity.compact,
                    ),
                    Chip(
                      label: Text('${_installedIds.length} installed'),
                      visualDensity: VisualDensity.compact,
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: _browseLocalGguf,
                        icon: const Icon(Icons.folder_open),
                        label: const Text('Browse local GGUF'),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: ElevatedButton.icon(
                        onPressed: _applyAutoSelect,
                        icon: const Icon(Icons.auto_fix_high),
                        label: const Text('Apply load order'),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: _accent.withValues(alpha: 0.2),
                          foregroundColor: _accent,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        _glassCard(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('System RAM', style: TextStyle(color: Colors.grey)),
                const SizedBox(height: 8),
                LinearProgressIndicator(
                  value: ramPct,
                  backgroundColor: Colors.white12,
                  color: ramPct > 0.85 ? Colors.orange : _accent,
                ),
                const SizedBox(height: 6),
                Text(
                  '${ramUsed.toStringAsFixed(1)} / ${ramTotal.toStringAsFixed(1)} GB used',
                  style: const TextStyle(fontSize: 12, color: Colors.grey),
                ),
                const SizedBox(height: 8),
                const Text(
                  'Models with "respect RAM estimate" skip themselves when your machine is too tight.',
                  style: TextStyle(fontSize: 11, color: Colors.grey),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        _glassCard(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Next auto-load match', style: TextStyle(color: Colors.grey)),
                const SizedBox(height: 8),
                if (_nextMatch != null) ...[
                  Text(
                    _nextMatch!.label,
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _nextMatch!.reason,
                    style: const TextStyle(fontSize: 12, color: Colors.grey),
                  ),
                ] else
                  const Text(
                    'No installed model matches your load order yet. Download models or adjust conditions.',
                    style: TextStyle(fontSize: 12, color: Colors.orange),
                  ),
                const SizedBox(height: 12),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: const Text('Auto-load on startup'),
                  subtitle: const Text(
                    'When no model is active, pick the first matching entry from Load order.',
                    style: TextStyle(fontSize: 11),
                  ),
                  value: _preferences?.autoSelect ?? false,
                  onChanged: (v) {
                    if (_preferences == null) return;
                    setState(() {
                      _preferences = catalog.ModelPreferencesFrb(
                        autoSelect: v,
                        entries: _preferences!.entries,
                      );
                      _prefsDirty = true;
                    });
                  },
                ),
              ],
            ),
          ),
        ),
        if (_downloads.values.any((d) => _isDownloadInProgress(d.id)))
          ..._downloads.values
              .where((d) => _isDownloadInProgress(d.id))
              .map(_buildDownloadBanner),
      ],
    );
  }

  Widget _buildPriorityTab() {
    final entries = _preferences?.entries ?? [];
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Text(
            'Drag to reorder. #1 is tried first. Each row only loads when its condition is met and the GGUF is installed.',
            style: TextStyle(color: Colors.grey.shade400, fontSize: 12),
          ),
        ),
        Expanded(
          child: entries.isEmpty
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Icon(Icons.queue, size: 48, color: Colors.grey),
                      const SizedBox(height: 12),
                      const Text('No models in load order'),
                      const SizedBox(height: 8),
                      TextButton(
                        onPressed: () => _tabs.animateTo(2),
                        child: const Text('Add from Catalog tab'),
                      ),
                    ],
                  ),
                )
              : ReorderableListView.builder(
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  itemCount: entries.length,
                  onReorder: _reorderPreference,
                  itemBuilder: (context, index) {
                    final e = entries[index];
                    final installed = _installedIds.contains(e.modelId);
                    return _glassCard(
                      key: ValueKey(e.modelId),
                      margin: const EdgeInsets.only(bottom: 10),
                      child: ListTile(
                        leading: CircleAvatar(
                          backgroundColor: _accent.withValues(alpha: 0.15),
                          child: Text(
                            '${e.priority}',
                            style: const TextStyle(
                              color: _accent,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ),
                        title: Text(
                          e.label,
                          style: const TextStyle(color: Colors.white),
                        ),
                        subtitle: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              installed ? 'Installed' : 'Not installed — download from Catalog',
                              style: TextStyle(
                                fontSize: 11,
                                color: installed ? Colors.greenAccent : Colors.orange,
                              ),
                            ),
                            const SizedBox(height: 6),
                            DropdownButtonFormField<String>(
                              value: e.when.task,
                              dropdownColor: const Color(0xFF12121A),
                              decoration: const InputDecoration(
                                labelText: 'Load when',
                                isDense: true,
                                border: OutlineInputBorder(),
                              ),
                              items: _taskOptions.entries
                                  .map(
                                    (kv) => DropdownMenuItem(
                                      value: kv.key,
                                      child: Text(kv.value, style: const TextStyle(fontSize: 12)),
                                    ),
                                  )
                                  .toList(),
                              onChanged: (v) {
                                if (v != null) _updateEntryCondition(index, v);
                              },
                            ),
                          ],
                        ),
                        trailing: IconButton(
                          icon: const Icon(Icons.delete_outline, color: Colors.redAccent),
                          onPressed: () => _removePreference(index),
                        ),
                      ),
                    );
                  },
                ),
        ),
        if (_prefsDirty)
          Padding(
            padding: const EdgeInsets.all(16),
            child: SizedBox(
              width: double.infinity,
              child: ElevatedButton.icon(
                onPressed: _savePreferences,
                icon: const Icon(Icons.save),
                label: const Text('Save load order'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: _accent.withValues(alpha: 0.2),
                  foregroundColor: _accent,
                ),
              ),
            ),
          ),
      ],
    );
  }

  Widget _buildCatalogTab(String activeModel) {
    final allTags = <String>{};
    for (final m in _models) {
      allTags.addAll(m.tags);
    }

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: TextField(
            style: const TextStyle(color: Colors.white),
            decoration: InputDecoration(
              hintText: 'Search models...',
              prefixIcon: const Icon(Icons.search),
              filled: true,
              fillColor: Colors.white.withValues(alpha: 0.05),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
            ),
            onChanged: (v) {
              _searchQuery = v;
              setState(_applyFilters);
            },
          ),
        ),
        SizedBox(
          height: 44,
          child: ListView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            children: [
              FilterChip(
                label: const Text('All'),
                selected: _selectedTag == null,
                onSelected: (_) {
                  _selectedTag = null;
                  setState(_applyFilters);
                },
              ),
              ...allTags.map(
                (tag) => Padding(
                  padding: const EdgeInsets.only(left: 6),
                  child: FilterChip(
                    label: Text(tag),
                    selected: _selectedTag == tag,
                    onSelected: (_) {
                      _selectedTag = tag;
                      setState(_applyFilters);
                    },
                  ),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: ListView.builder(
            padding: const EdgeInsets.all(12),
            itemCount: _filteredModels.length,
            itemBuilder: (context, index) {
              final model = _filteredModels[index];
              final isActive = _isActiveModel(model, activeModel);
              final download = _downloads[model.id];
              final downloading =
                  download != null && _isDownloadInProgress(model.id);
              final inQueue = _preferences?.entries
                      .any((e) => e.modelId == model.id) ??
                  false;

              return _glassCard(
                margin: const EdgeInsets.only(bottom: 10),
                child: Padding(
                  padding: const EdgeInsets.all(14),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  model.name,
                                  style: const TextStyle(
                                    color: Colors.white,
                                    fontWeight: FontWeight.bold,
                                    fontSize: 15,
                                  ),
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  '${model.provider} · ${model.sizeMb} MB · ${model.quantization}',
                                  style: const TextStyle(
                                    fontSize: 11,
                                    color: Colors.grey,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          if (isActive)
                            const Chip(
                              label: Text('ACTIVE'),
                              backgroundColor: Color(0x331E8E3E),
                              labelStyle: TextStyle(color: Colors.greenAccent),
                              visualDensity: VisualDensity.compact,
                            )
                          else if (model.isDownloaded)
                            const Chip(
                              label: Text('Installed'),
                              visualDensity: VisualDensity.compact,
                            ),
                        ],
                      ),
                      if (model.tags.isNotEmpty) ...[
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 6,
                          runSpacing: 4,
                          children: model.tags
                              .take(5)
                              .map(
                                (t) => Chip(
                                  label: Text(t),
                                  visualDensity: VisualDensity.compact,
                                  labelStyle: const TextStyle(fontSize: 10),
                                ),
                              )
                              .toList(),
                        ),
                      ],
                      if (model.isMultimodal)
                        const Padding(
                          padding: EdgeInsets.only(top: 6),
                          child: Text(
                            'Multimodal — needs mmproj for vision',
                            style: TextStyle(fontSize: 10, color: _accent),
                          ),
                        ),
                      if (downloading) ...[
                        const SizedBox(height: 10),
                        LinearProgressIndicator(
                          value: download.totalBytes > BigInt.zero
                              ? (download.progress / 100).clamp(0.0, 1.0)
                              : null,
                        ),
                        Text(
                          '${download.progress.toStringAsFixed(0)}% · ${download.status}',
                          style: const TextStyle(fontSize: 10, color: Colors.grey),
                        ),
                      ],
                      const SizedBox(height: 10),
                      Row(
                        children: [
                          if (!inQueue)
                            TextButton.icon(
                              onPressed: () => _addToQueue(model),
                              icon: const Icon(Icons.add, size: 16),
                              label: const Text('Add to order'),
                            ),
                          const Spacer(),
                          if (model.isDownloaded) ...[
                            IconButton(
                              tooltip: 'Remove',
                              onPressed: () => _removeModel(model),
                              icon: const Icon(Icons.delete_outline, size: 20),
                            ),
                            OutlinedButton(
                              onPressed: () => _setActiveModel(model),
                              child: Text(isActive ? 'Active' : 'Use now'),
                            ),
                          ] else if (downloading)
                            TextButton(
                              onPressed: () => _cancelDownload(model.id),
                              child: const Text('Cancel'),
                            )
                          else
                            ElevatedButton(
                              onPressed: () => _downloadModel(model),
                              style: ElevatedButton.styleFrom(
                                backgroundColor: _accent.withValues(alpha: 0.15),
                                foregroundColor: _accent,
                              ),
                              child: const Text('Download'),
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }

  Widget _buildDownloadBanner(api.ProgressPayload download) {
    final matches =
        _models.where((m) => m.id == download.id).map((m) => m.name).toList();
    final name = matches.isEmpty ? download.id : matches.first;
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: _glassCard(
        child: ListTile(
          title: Text('$name — ${download.status}'),
          subtitle: LinearProgressIndicator(
            value: download.totalBytes > BigInt.zero
                ? (download.progress / 100).clamp(0.0, 1.0)
                : null,
          ),
          trailing: TextButton(
            onPressed: () => _cancelDownload(download.id),
            child: const Text('Cancel'),
          ),
        ),
      ),
    );
  }

  Widget _glassCard({required Widget child, EdgeInsetsGeometry? margin, Key? key}) {
    return Container(
      key: key,
      margin: margin,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(14),
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: 8, sigmaY: 8),
          child: Container(
            decoration: BoxDecoration(
              color: Colors.white.withValues(alpha: 0.04),
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
            ),
            child: child,
          ),
        ),
      ),
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
  final int ramEstimateMb;
  final List<String> tags;
  final List<String> recommendedFor;
  final String? downloadUrl;
  final String? notes;
  bool isDownloaded;
  final bool isMultimodal;
  final String? architecture;
  final int? contextWindow;
  final String? localGgufPath;

  LLMModel({
    required this.id,
    required this.name,
    required this.provider,
    required this.sizeMb,
    required this.ramEstimateMb,
    required this.quantization,
    required this.license,
    required this.tags,
    required this.recommendedFor,
    this.downloadUrl,
    this.notes,
    this.isDownloaded = false,
    this.isMultimodal = false,
    this.architecture,
    this.contextWindow,
    this.localGgufPath,
  });
}
