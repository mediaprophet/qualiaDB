import 'dart:ui';
import 'dart:async';
import 'package:flutter/material.dart';
import '../src/rust/api/qualia_api.dart';

class LLMHubScreen extends StatefulWidget {
  const LLMHubScreen({super.key});

  @override
  State<LLMHubScreen> createState() => _LLMHubScreenState();
}

class _LLMHubScreenState extends State<LLMHubScreen> {
  List<Map<String, dynamic>> _models = [];
  String _loadingModelId = '';
  Map<String, ProgressPayload> _activeDownloads = {};
  Timer? _timer;

  HardwareStatus? _hardwareStatus;

  @override
  void initState() {
    super.initState();
    _loadModels();
    _loadHardware();
    _timer = Timer.periodic(const Duration(milliseconds: 500), (t) async {
      final list = await getActiveDownloads();
      if (mounted) {
        setState(() {
          _activeDownloads = { for (var e in list) e.id : e };
        });
      }
    });
  }

  Future<void> _loadHardware() async {
    final status = await getHardwareStatus();
    if (mounted) {
      setState(() {
        _hardwareStatus = status;
      });
    }
  }

  Future<void> _loadModels() async {
    final remoteCatalog = await fetchModelCatalogReal();
    final localModels = await discoverModels();
    
    setState(() {
      _models.clear();
      for (var rc in remoteCatalog) {
        _models.add({
          'id': rc.id,
          'name': rc.name,
          'tag': rc.tag,
          'params': rc.params ?? '?',
          'format': rc.format,
          'size': rc.size,
          'vram': rc.vram ?? '?',
          'installed': false,
          'active': false,
        });
      }

      for (var local in localModels) {
        try {
          var matched = _models.firstWhere((m) => local.name.contains(m['id']));
          matched['installed'] = true;
          if (local.isActive) {
            matched['active'] = true;
          }
        } catch (e) {
          _models.add({
            'id': local.name,
            'name': local.name,
            'tag': 'Local',
            'params': '?',
            'format': 'GGUF',
            'size': '?',
            'vram': '?',
            'installed': true,
            'active': local.isActive,
          });
        }
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  void _handleSetActive(String id) async {
    setState(() => _loadingModelId = id);
    await setActiveModel(modelName: id);
    
    if (mounted) {
      setState(() {
        for (var m in _models) { m['active'] = false; }
        _models.firstWhere((m) => m['id'] == id)['active'] = true;
        _loadingModelId = '';
      });
    }
  }

  void _handleDownload(String id) {
    // We pass a dummy URL since it's just testing FFI
    downloadModel(url: "https://huggingface.co/dummy", filename: "\$id.gguf", modelId: id);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Telemetry Row
          Row(
            children: [
              _buildTelemetryCard(
                'System RAM', 
                _hardwareStatus != null ? '\${_hardwareStatus!.ramTotalGb} GB Total' : '...', 
                _hardwareStatus != null ? '\${_hardwareStatus!.ramUsedGb} GB Used' : '...', 
                Icons.memory, 
                const Color(0xFF00FF88)
              ),
              const SizedBox(width: 16),
              _buildTelemetryCard(
                'Est. VRAM', 
                _hardwareStatus != null ? '\${_hardwareStatus!.vramEstimatedGb} GB' : '...', 
                'Available', 
                Icons.speed, 
                const Color(0xFFFFD700)
              ),
              const SizedBox(width: 16),
              Expanded(
                child: _buildGlassContainer(
                  child: Padding(
                    padding: const EdgeInsets.all(16.0),
                    child: Row(
                      children: [
                        const Icon(Icons.bolt, color: Color(0xFFB026FF), size: 32),
                        const SizedBox(width: 16),
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('ACTIVE MODEL', style: TextStyle(color: Colors.grey, fontSize: 10, fontFamily: 'monospace', letterSpacing: 1.2)),
                            const SizedBox(height: 4),
                            Text(
                              _models.firstWhere((m) => m['active'], orElse: () => {'name': 'None Loaded'})['name'],
                              style: const TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold, fontFamily: 'monospace'),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),
          
          Expanded(
            child: _buildGlassContainer(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Padding(
                    padding: EdgeInsets.all(24.0),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(Icons.hub, color: Color(0xFF00F0FF), size: 24),
                            SizedBox(width: 12),
                            Text('Model Hub — GGUF (HuggingFace)', style: TextStyle(color: Colors.white, fontSize: 20, fontWeight: FontWeight.bold)),
                          ],
                        ),
                        SizedBox(height: 8),
                        Text('Open-weight models downloaded to your local Models directory. Set one as active to use it in Neuro-Chat.', style: TextStyle(color: Colors.grey, fontSize: 12)),
                      ],
                    ),
                  ),
                  Expanded(
                    child: ListView.builder(
                      padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 8.0),
                      itemCount: _models.length,
                      itemBuilder: (context, index) {
                        final m = _models[index];
                        final isActive = m['active'];
                        final isInstalled = m['installed'];
                        final isLoading = _loadingModelId == m['id'];
                        
                        return Container(
                          margin: const EdgeInsets.only(bottom: 16),
                          padding: const EdgeInsets.all(16),
                          decoration: BoxDecoration(
                            color: isActive ? const Color(0xFF00FF88).withOpacity(0.05) : Colors.black.withOpacity(0.4),
                            borderRadius: BorderRadius.circular(12),
                            border: Border.all(color: isActive ? const Color(0xFF00FF88).withOpacity(0.3) : Colors.white.withOpacity(0.05)),
                            boxShadow: isActive ? [BoxShadow(color: const Color(0xFF00FF88).withOpacity(0.05), blurRadius: 10)] : [],
                          ),
                          child: Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Row(
                                    children: [
                                      Text(m['name'], style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16)),
                                      const SizedBox(width: 12),
                                      Container(
                                        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                        decoration: BoxDecoration(color: Colors.white.withOpacity(0.1), borderRadius: BorderRadius.circular(4)),
                                        child: Text(m['tag'], style: const TextStyle(color: Colors.white70, fontSize: 10, fontWeight: FontWeight.bold)),
                                      ),
                                      if (isActive) ...[
                                        const SizedBox(width: 8),
                                        Container(
                                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                          decoration: BoxDecoration(color: const Color(0xFF00FF88).withOpacity(0.2), borderRadius: BorderRadius.circular(4), border: Border.all(color: const Color(0xFF00FF88).withOpacity(0.5))),
                                          child: const Row(
                                            children: [
                                              Icon(Icons.check_circle, color: Color(0xFF00FF88), size: 10),
                                              SizedBox(width: 4),
                                              Text('ACTIVE', style: TextStyle(color: Color(0xFF00FF88), fontSize: 10, fontWeight: FontWeight.bold)),
                                            ],
                                          ),
                                        )
                                      ]
                                    ],
                                  ),
                                  const SizedBox(height: 8),
                                  Row(
                                    children: [
                                      Text('${m['params']} params', style: const TextStyle(color: Colors.grey, fontFamily: 'monospace', fontSize: 10)),
                                      const SizedBox(width: 12),
                                      Text(m['format'], style: const TextStyle(color: Colors.grey, fontFamily: 'monospace', fontSize: 10)),
                                      const SizedBox(width: 12),
                                      Text(m['size'], style: const TextStyle(color: Colors.grey, fontFamily: 'monospace', fontSize: 10)),
                                      const SizedBox(width: 12),
                                      Text('≥ ${m['vram']} VRAM', style: const TextStyle(color: Color(0xFF00F0FF), fontFamily: 'monospace', fontSize: 10)),
                                    ],
                                  ),
                                ],
                              ),
                              Row(
                                children: [
                                  if (isInstalled && !isActive)
                                    ElevatedButton.icon(
                                      onPressed: isLoading ? null : () => _handleSetActive(m['id']),
                                      icon: Icon(isLoading ? Icons.sync : Icons.play_arrow, size: 16),
                                      label: Text(isLoading ? 'Loading...' : 'Load'),
                                      style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF00FF88).withOpacity(0.1), foregroundColor: const Color(0xFF00FF88), side: BorderSide(color: const Color(0xFF00FF88).withOpacity(0.3))),
                                    )
                                  else if (!isInstalled)
                                    if (_activeDownloads.containsKey(m['id']))
                                      Row(
                                        children: [
                                          SizedBox(
                                            width: 100,
                                            child: LinearProgressIndicator(
                                              value: _activeDownloads[m['id']]!.progress / 100.0,
                                              backgroundColor: Colors.white10,
                                              valueColor: const AlwaysStoppedAnimation<Color>(Color(0xFF00F0FF)),
                                            ),
                                          ),
                                          const SizedBox(width: 8),
                                          IconButton(
                                            icon: const Icon(Icons.cancel, color: Colors.redAccent, size: 20),
                                            onPressed: () => cancelDownload(id: m['id']),
                                          )
                                        ]
                                      )
                                    else
                                      ElevatedButton.icon(
                                        onPressed: () => _handleDownload(m['id']),
                                        icon: const Icon(Icons.download, size: 16),
                                        label: const Text('Download'),
                                        style: ElevatedButton.styleFrom(backgroundColor: Colors.white.withOpacity(0.05), foregroundColor: Colors.white, side: const BorderSide(color: Colors.white10)),
                                      )
                                ],
                              ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTelemetryCard(String title, String val1, String val2, IconData icon, Color color) {
    return Expanded(
      child: _buildGlassContainer(
        child: Padding(
          padding: const EdgeInsets.all(16.0),
          child: Row(
            children: [
              Icon(icon, color: color, size: 32),
              const SizedBox(width: 16),
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title.toUpperCase(), style: const TextStyle(color: Colors.grey, fontSize: 10, fontFamily: 'monospace', letterSpacing: 1.2)),
                  const SizedBox(height: 4),
                  Row(
                    children: [
                      Text(val1, style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold)),
                      const SizedBox(width: 4),
                      Text(val2, style: const TextStyle(color: Colors.grey, fontSize: 14)),
                    ],
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildGlassContainer({required Widget child}) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white.withOpacity(0.03),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: Colors.white.withOpacity(0.1)),
          ),
          child: child,
        ),
      ),
    );
  }
}
