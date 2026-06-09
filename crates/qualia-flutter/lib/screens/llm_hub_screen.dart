import 'dart:async';
import 'dart:convert';
import 'dart:ui';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../main.dart' show activeModelPathProvider;
import '../services/model_activation_service.dart';
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
  StreamSubscription<String>? _telemetrySubscription;
  LlmLoadStatus? _loadStatus;
  List<LlmLoadStatus> _loadHistory = const [];
  LlmRuntimeSnapshot _runtimeSnapshot = const LlmRuntimeSnapshot();
  bool _isActivatingModel = false;
  api.InferenceBackendSettingsFrb? _inferenceBackend;

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
    _ensureTelemetryStream();
    _refreshAll();
    _restoreActiveDownloads();
  }

  @override
  void dispose() {
    _downloadPollTimer?.cancel();
    _telemetrySubscription?.cancel();
    _tabs.dispose();
    super.dispose();
  }

  void _ensureTelemetryStream() {
    _telemetrySubscription ??= api.initTelemetryStream().listen(
      _handleTelemetryLine,
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('LLM Hub telemetry stream error: $error');
      },
      cancelOnError: false,
    );
  }

  void _beginModelActivation(String message) {
    final queued = LlmLoadStatus(
      stage: 'queued',
      progress: 0,
      message: message,
      rawLine: message,
    );
    setState(() {
      _isActivatingModel = true;
      _loadStatus = queued;
      _loadHistory = [queued];
      _runtimeSnapshot = const LlmRuntimeSnapshot();
    });
  }

  void _completeModelActivation(String message) {
    if (!mounted) return;
    setState(() {
      _isActivatingModel = false;
      if (_loadStatus == null || !_loadStatus!.isTerminal) {
        final completed = LlmLoadStatus(
          stage: 'active',
          progress: 1,
          message: message,
          rawLine: message,
        );
        final nextHistory = [..._loadHistory, completed];
        if (nextHistory.length > 8) {
          nextHistory.removeRange(0, nextHistory.length - 8);
        }
        _loadStatus = completed;
        _loadHistory = nextHistory;
      }
    });
  }

  void _handleTelemetryLine(String line) {
    final status = LlmLoadStatus.tryParse(line);
    if (status == null || !mounted) return;
    final nextHistory = [..._loadHistory];
    if (nextHistory.isEmpty || nextHistory.last.rawLine != status.rawLine) {
      nextHistory.add(status);
      if (nextHistory.length > 8) {
        nextHistory.removeRange(0, nextHistory.length - 8);
      }
    }
    setState(() {
      _loadStatus = status;
      _loadHistory = nextHistory;
      _runtimeSnapshot = _runtimeSnapshot.consume(status);
      _isActivatingModel = !status.isTerminal;
    });
  }

  Future<void> _refreshAll() async {
    setState(() => _isLoading = true);
    try {
      final resources = await catalog.loadLlmResources();
      final local = await api.discoverModels();
      final activeModel = await api.getActiveModel();
      final inferenceBackend = await api.getInferenceBackendSettings();
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
        final isFullPath = entry.name.contains(RegExp(r'[\\/]'));
        _ensureLocalModelEntry(
          fileName: isFullPath
              ? entry.name.split(RegExp(r'[\\/]')).last
              : entry.name,
          fullPath: isFullPath ? entry.name : null,
        );
      }
      if (activeModel != null && activeModel.isNotEmpty) {
        _ensureLocalModelEntry(
          fileName: activeModel.split(RegExp(r'[\\/]')).last,
          fullPath: activeModel.contains(RegExp(r'[\\/]')) ? activeModel : null,
        );
      }

      _applyFilters();
      _prefsDirty = false;
      _inferenceBackend = inferenceBackend;
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
    _beginModelActivation('Selecting the best installed model…');
    try {
      await catalog.applyModelPreferenceAsync(task: 'chat');
      await waitForModelActivation();
      final active = await api.getActiveModel();
      if (active != null) {
        ref.read(activeModelPathProvider.notifier).state = active;
      }
      await _refreshAll();
      _completeModelActivation(
        _nextMatch != null
            ? 'Loaded ${_nextMatch!.label}'
            : 'Applied best matching model',
      );
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
        setState(() {
          _isActivatingModel = false;
          _loadStatus = LlmLoadStatus.failed('Auto-load failed: $e');
        });
      }
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
    _beginModelActivation('Activating ${model.name}…');
    try {
      if (model.localGgufPath != null && model.localGgufPath!.isNotEmpty) {
        await activateModelAsync(model.localGgufPath!);
        ref.read(activeModelPathProvider.notifier).state = model.localGgufPath!;
        _ensureLocalModelEntry(
          fileName: model.localGgufPath!.split(RegExp(r'[\\/]')).last,
          fullPath: model.localGgufPath,
        );
        _applyFilters();
      } else {
        final modelName = model.id.startsWith('local-')
            ? '${model.name}.gguf'
            : '${model.id}.gguf';
        await activateModelAsync(modelName);
        final active = await api.getActiveModel();
        ref.read(activeModelPathProvider.notifier).state =
            active ?? modelName;
      }
      await _refreshAll();
      _completeModelActivation('${model.name} is now active');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${model.name} is now active')),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isActivatingModel = false;
          _loadStatus = LlmLoadStatus.failed('Could not activate ${model.name}: $e');
        });
      }
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not activate: $e')),
        );
      }
    }
  }

  Future<void> _activateLocalPath(String path) async {
    await activateModelAsync(path);
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
    _beginModelActivation('Activating ${result!.files.single.name}…');
    try {
      await _activateLocalPath(path);
      await _refreshAll();
      _completeModelActivation('${result!.files.single.name} is now active');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Active: ${result!.files.single.name}')),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isActivatingModel = false;
          _loadStatus = LlmLoadStatus.failed('Could not activate local GGUF: $e');
        });
      }
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
      final active = await api.getActiveModel();
      if (active != null && active.isNotEmpty) {
        ref.read(activeModelPathProvider.notifier).state = active;
      }
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

  Future<void> _unloadActiveModel() async {
    final active = ref.read(activeModelPathProvider);
    if (active.isEmpty) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('No model is currently loaded')),
        );
      }
      return;
    }
    _beginModelActivation('Unloading resident model…');
    try {
      await api.unloadActiveModel();
      ref.read(activeModelPathProvider.notifier).state = '';
      await _refreshAll();
      _completeModelActivation('Model memory released');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Model unloaded from memory')),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isActivatingModel = false;
          _loadStatus = LlmLoadStatus.failed('Unload failed: $e');
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Unload failed: $e')),
        );
      }
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
    final gpuAvailable = _runtimeSnapshot.availableVramGb ?? _hardware?.vramEstimatedGb;
    final gpuTelemetryKnown = gpuAvailable != null && gpuAvailable > 0;
    final engineLabel = _runtimeSnapshot.backendLabel == null
        ? 'native GGUF + wgpu/DirectML (in-process)'
        : 'native GGUF + ${_runtimeSnapshot.backendLabel}';

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
                  'Engine: $engineLabel',
                  style: TextStyle(color: Colors.grey.shade500, fontSize: 11),
                ),
                if (_inferenceBackend != null &&
                    _inferenceBackend!.backend != 'local') ...[
                  const SizedBox(height: 4),
                  Text(
                    'Backend preference: ${_inferenceBackend!.backend}'
                    '${_inferenceBackend!.remoteEndpoint.isNotEmpty ? ' → ${_inferenceBackend!.remoteEndpoint}' : ''}',
                    style: TextStyle(color: Colors.amber.shade200, fontSize: 11),
                  ),
                ],
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
                    if (_runtimeSnapshot.backendLabel != null)
                      Chip(
                        label: Text(_runtimeSnapshot.backendLabel!),
                        visualDensity: VisualDensity.compact,
                      ),
                    if (_runtimeSnapshot.memoryRouteLabel != null)
                      Chip(
                        label: Text(_runtimeSnapshot.memoryRouteLabel!),
                        visualDensity: VisualDensity.compact,
                      ),
                  ],
                ),
                const SizedBox(height: 16),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: _isActivatingModel ? null : _browseLocalGguf,
                        icon: const Icon(Icons.folder_open),
                        label: const Text('Browse local GGUF'),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: activeModel.isEmpty || _isActivatingModel
                            ? null
                            : _unloadActiveModel,
                        icon: const Icon(Icons.eject),
                        label: const Text('Unload model'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: ElevatedButton.icon(
                        onPressed: _isActivatingModel ? null : _applyAutoSelect,
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
        if (_loadStatus != null) ...[
          const SizedBox(height: 16),
          _buildLoadProgressCard(_loadStatus!),
        ],
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
                const Text(
                  'GPU / VRAM',
                  style: TextStyle(color: Colors.grey),
                ),
                const SizedBox(height: 8),
                Text(
                  gpuTelemetryKnown
                      ? '${gpuAvailable!.toStringAsFixed(1)} GB currently available'
                      : 'Telemetry unavailable on this backend',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  _runtimeSnapshot.adapterLabel ??
                      'Windows + DirectML reports live local VRAM budget; other backends fall back to stream telemetry only.',
                  style: const TextStyle(fontSize: 11, color: Colors.grey),
                ),
                if (_runtimeSnapshot.sharedFreeGb != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    'Shared system memory free: ${_runtimeSnapshot.sharedFreeGb!.toStringAsFixed(1)} GB',
                    style: const TextStyle(fontSize: 12, color: Colors.grey),
                  ),
                ],
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
                              onPressed: _isActivatingModel ? null : () => _setActiveModel(model),
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

  Widget _buildLoadProgressCard(LlmLoadStatus status) {
    final barColor = status.isError
        ? Colors.redAccent
        : status.isTerminal
            ? Colors.greenAccent
            : _accent;
    final value = status.isError ? 1.0 : status.progress.clamp(0.0, 1.0);
    final recentStages = _loadHistory.length <= 4
        ? _loadHistory
        : _loadHistory.sublist(_loadHistory.length - 4);

    return _glassCard(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  status.isError
                      ? Icons.error_outline
                      : status.isTerminal
                          ? Icons.check_circle_outline
                          : Icons.sync,
                  color: barColor,
                ),
                const SizedBox(width: 10),
                Text(
                  status.isError ? 'Model load failed' : 'Model load pipeline',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const Spacer(),
                Text(
                  '${(value * 100).round()}%',
                  style: TextStyle(color: barColor, fontFamily: 'monospace'),
                ),
              ],
            ),
            const SizedBox(height: 12),
            LinearProgressIndicator(
              value: status.isTerminal && !status.isError ? 1.0 : value,
              minHeight: 10,
              color: barColor,
              backgroundColor: Colors.white12,
            ),
            const SizedBox(height: 10),
            Text(
              status.message,
              style: const TextStyle(color: Colors.white),
            ),
            const SizedBox(height: 4),
            Text(
              'Stage: ${status.stageLabel}',
              style: const TextStyle(fontSize: 11, color: Colors.grey),
            ),
            if (_runtimeSnapshot.backendLabel != null ||
                _runtimeSnapshot.availableRamGb != null ||
                _runtimeSnapshot.availableVramGb != null ||
                _runtimeSnapshot.mappedModelGb != null ||
                _runtimeSnapshot.kvCacheMb != null) ...[
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  if (_runtimeSnapshot.backendLabel != null)
                    _statusChip('Backend ${_runtimeSnapshot.backendLabel!}'),
                  if (_runtimeSnapshot.memoryRouteLabel != null)
                    _statusChip(_runtimeSnapshot.memoryRouteLabel!),
                  if (_runtimeSnapshot.availableRamGb != null &&
                      _runtimeSnapshot.totalRamGb != null)
                    _statusChip(
                      'RAM free ${_runtimeSnapshot.availableRamGb!.toStringAsFixed(1)}/${_runtimeSnapshot.totalRamGb!.toStringAsFixed(1)} GB',
                    ),
                  if (_runtimeSnapshot.availableVramGb != null)
                    _statusChip(
                      _runtimeSnapshot.totalVramGb != null
                          ? 'VRAM free ${_runtimeSnapshot.availableVramGb!.toStringAsFixed(1)}/${_runtimeSnapshot.totalVramGb!.toStringAsFixed(1)} GB'
                          : 'VRAM free ${_runtimeSnapshot.availableVramGb!.toStringAsFixed(1)} GB',
                    ),
                  if (_runtimeSnapshot.mappedModelGb != null)
                    _statusChip(
                      'Mapped ${_runtimeSnapshot.mappedModelGb!.toStringAsFixed(2)} GB',
                    ),
                  if (_runtimeSnapshot.kvCacheMb != null)
                    _statusChip('KV ${_runtimeSnapshot.kvCacheMb} MiB'),
                ],
              ),
            ],
            if (recentStages.isNotEmpty) ...[
              const SizedBox(height: 14),
              const Text(
                'Recent stages',
                style: TextStyle(fontSize: 11, color: Colors.grey),
              ),
              const SizedBox(height: 6),
              ...recentStages.map(
                (entry) => Padding(
                  padding: const EdgeInsets.only(bottom: 4),
                  child: Text(
                    '${entry.stageLabel}: ${entry.message}',
                    style: const TextStyle(fontSize: 11, color: Colors.white70),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _statusChip(String label) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.05),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
      ),
      child: Text(
        label,
        style: const TextStyle(fontSize: 11, color: Colors.white70),
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

class LlmLoadStatus {
  const LlmLoadStatus({
    required this.stage,
    required this.progress,
    required this.message,
    required this.rawLine,
  });

  final String stage;
  final double progress;
  final String message;
  final String rawLine;

  bool get isError => stage == 'failed';
  bool get isTerminal => stage == 'active' || stage == 'failed';

  String get stageLabel => stage.replaceAll('-', ' ');

  static final RegExp _pattern =
      RegExp(r'LLM_LOAD\|([^|]+)\|([0-9.]+)\|(.*)$');

  static LlmLoadStatus? tryParse(String line) {
    final match = _pattern.firstMatch(line);
    if (match == null) return null;
    return LlmLoadStatus(
      stage: match.group(1) ?? 'unknown',
      progress: double.tryParse(match.group(2) ?? '') ?? 0,
      message: (match.group(3) ?? line).trim(),
      rawLine: line,
    );
  }

  factory LlmLoadStatus.failed(String message) => LlmLoadStatus(
        stage: 'failed',
        progress: 1,
        message: message,
        rawLine: message,
      );
}

class LlmRuntimeSnapshot {
  const LlmRuntimeSnapshot({
    this.backendLabel,
    this.adapterLabel,
    this.memoryRouteLabel,
    this.availableRamGb,
    this.totalRamGb,
    this.availableVramGb,
    this.totalVramGb,
    this.sharedFreeGb,
    this.mappedModelGb,
    this.kvCacheMb,
  });

  final String? backendLabel;
  final String? adapterLabel;
  final String? memoryRouteLabel;
  final double? availableRamGb;
  final double? totalRamGb;
  final double? availableVramGb;
  final double? totalVramGb;
  final double? sharedFreeGb;
  final double? mappedModelGb;
  final int? kvCacheMb;

  static final RegExp _ramPattern = RegExp(
    r'System RAM ([0-9.]+)/([0-9.]+) GiB used; ([0-9.]+) GiB available',
  );
  static final RegExp _vramPattern = RegExp(
    r'VRAM free ([0-9.]+)/([0-9.]+) GiB on (.+) \(usage ([0-9.]+) GiB, shared free ([0-9.]+) GiB\)',
  );
  static final RegExp _ramMapPattern = RegExp(
    r'Mapped ([0-9.]+) GiB GGUF into system memory',
  );
  static final RegExp _placementPattern = RegExp(
    r'Model mapped in system RAM \(([0-9.]+) GiB\) and KV cache reserved.*\(([0-9]+) MiB\)',
  );
  static final RegExp _kvPattern = RegExp(r'KV (?:cache )?(?:reserved )?\(?([0-9]+) MiB');
  static final RegExp _kvReservePattern = RegExp(
    r'Reserved ([0-9.]+) MiB KV cache',
  );

  LlmRuntimeSnapshot consume(LlmLoadStatus status) {
    final message = status.message;
    var next = this;

    final ramMatch = _ramPattern.firstMatch(message);
    if (ramMatch != null) {
      final used = double.tryParse(ramMatch.group(1) ?? '');
      final total = double.tryParse(ramMatch.group(2) ?? '');
      final free = double.tryParse(ramMatch.group(3) ?? '');
      next = next.copyWith(
        totalRamGb: total,
        availableRamGb: free,
        memoryRouteLabel: 'Loading via system RAM',
      );
      if (used != null && total != null && free == null) {
        next = next.copyWith(
          availableRamGb: (total - used).clamp(0.0, total).toDouble(),
        );
      }
    }

    final vramMatch = _vramPattern.firstMatch(message);
    if (vramMatch != null) {
      next = next.copyWith(
        availableVramGb: double.tryParse(vramMatch.group(1) ?? ''),
        totalVramGb: double.tryParse(vramMatch.group(2) ?? ''),
        adapterLabel: vramMatch.group(3)?.trim(),
        sharedFreeGb: double.tryParse(vramMatch.group(5) ?? ''),
      );
    }

    final mappedMatch = _ramMapPattern.firstMatch(message);
    if (mappedMatch != null) {
      next = next.copyWith(
        mappedModelGb: double.tryParse(mappedMatch.group(1) ?? ''),
        memoryRouteLabel: 'Mapped in system RAM',
      );
    }

    final placementMatch = _placementPattern.firstMatch(message);
    if (placementMatch != null) {
      next = next.copyWith(
        mappedModelGb: double.tryParse(placementMatch.group(1) ?? ''),
        kvCacheMb: int.tryParse(placementMatch.group(2) ?? ''),
        memoryRouteLabel: 'System RAM + VRAM cache',
      );
    }

    final kvMatch = _kvPattern.firstMatch(message);
    if (kvMatch != null) {
      next = next.copyWith(kvCacheMb: int.tryParse(kvMatch.group(1) ?? ''));
    }
    final kvReserveMatch = _kvReservePattern.firstMatch(message);
    if (kvReserveMatch != null) {
      next = next.copyWith(
        kvCacheMb: double.tryParse(kvReserveMatch.group(1) ?? '')?.round(),
      );
    }

    if (status.stage == 'gpu-backend' || status.stage == 'gpu-route') {
      if (message.contains('DirectML')) {
        next = next.copyWith(
          backendLabel: 'DirectML + wgpu',
          memoryRouteLabel: 'Streaming to VRAM',
        );
      } else if (message.contains('wgpu fallback')) {
        next = next.copyWith(
          backendLabel: 'wgpu fallback',
          memoryRouteLabel: 'Streaming to GPU',
        );
      }
    }

    if (status.stage == 'gpu-adapter' && next.adapterLabel == null) {
      next = next.copyWith(adapterLabel: message.replaceFirst('Using ', '').trim());
    }

    if (status.stage == 'active') {
      next = next.copyWith(memoryRouteLabel: 'Ready');
    }

    return next;
  }

  LlmRuntimeSnapshot copyWith({
    String? backendLabel,
    String? adapterLabel,
    String? memoryRouteLabel,
    double? availableRamGb,
    double? totalRamGb,
    double? availableVramGb,
    double? totalVramGb,
    double? sharedFreeGb,
    double? mappedModelGb,
    int? kvCacheMb,
  }) {
    return LlmRuntimeSnapshot(
      backendLabel: backendLabel ?? this.backendLabel,
      adapterLabel: adapterLabel ?? this.adapterLabel,
      memoryRouteLabel: memoryRouteLabel ?? this.memoryRouteLabel,
      availableRamGb: availableRamGb ?? this.availableRamGb,
      totalRamGb: totalRamGb ?? this.totalRamGb,
      availableVramGb: availableVramGb ?? this.availableVramGb,
      totalVramGb: totalVramGb ?? this.totalVramGb,
      sharedFreeGb: sharedFreeGb ?? this.sharedFreeGb,
      mappedModelGb: mappedModelGb ?? this.mappedModelGb,
      kvCacheMb: kvCacheMb ?? this.kvCacheMb,
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
