import 'dart:ui';
import 'package:flutter/material.dart';
import '../src/rust/api/qualia_api.dart';
import 'dart:math' as math;

class AssetLibraryScreen extends StatefulWidget {
  const AssetLibraryScreen({super.key});

  @override
  State<AssetLibraryScreen> createState() => _AssetLibraryScreenState();
}

class _AssetLibraryScreenState extends State<AssetLibraryScreen> with TickerProviderStateMixin {
  late AnimationController _spinnerController;
  late AnimationController _pulseController;
  
  String _pipelineState = 'idle'; // 'idle' or 'analyzing'
  String _typologyLens = 'Generic';

  final List<Map<String, dynamic>> _assets = [
    { 'id': '0x8F3BC122', 'type': 'Heraldry', 'facet': 'Lion Rampant | Or on Gules', 'origin': '14th Century', 'region': 'xywh=120,40,200,200', 'magnet': 'magnet:?xt=urn:btih:3f8a...', 'alpTokenId': 'alp:0x1A2...', 'isGhost': false },
    { 'id': '0x9E4CD233', 'type': 'Meme', 'facet': 'Distracted Boyfriend | Irony Tensor: 0.8', 'origin': '2015', 'region': 'xywh=0,0,1024,768', 'magnet': 'magnet:?xt=urn:btih:4a9b...', 'alpTokenId': null, 'isGhost': false },
    { 'id': '0xA15DE344', 'type': 'Hieroglyph', 'facet': 'Eye of Horus | Wedjat', 'origin': 'Ptolemaic', 'region': 'xywh=450,300,50,50', 'magnet': 'magnet:?xt=urn:btih:5b0c...', 'alpTokenId': 'alp:0x9B4...', 'isGhost': false }
  ];

  @override
  void initState() {
    super.initState();
    _spinnerController = AnimationController(vsync: this, duration: const Duration(seconds: 10))..repeat();
    _pulseController = AnimationController(vsync: this, duration: const Duration(seconds: 1))..repeat(reverse: true);
  }

  @override
  void dispose() {
    _spinnerController.dispose();
    _pulseController.dispose();
    super.dispose();
  }

  Future<void> _handleIngest() async {
    if (_pipelineState != 'idle') return;
    setState(() {
      _pipelineState = 'analyzing';
      _assets.insert(0, {
        'id': 'ghost-node',
        'type': _typologyLens == 'Generic' ? 'Analyzing...' : 'Extracting \$_typologyLens Facets...',
        'facet': 'Pending Vision Extraction',
        'origin': 'Unknown',
        'region': 'Pending...',
        'magnet': 'Initializing local seed...',
        'alpTokenId': null,
        'isGhost': true
      });
    });

    try {
      final res = await ingestLiterature(filePath: 'dummy_file.pdf');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(res)));
      }
      await upsertCmldDefinition(term: 'dummy_file', contextDid: 'did:qualia:context:123');
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Ingest Error: \$e')));
      }
    }

    if (!mounted) return;
    setState(() {
      _pipelineState = 'idle';
      _assets[0] = {
        'id': 'new_asset_\${DateTime.now().millisecondsSinceEpoch}',
        'type': _typologyLens == 'Generic' ? 'Unstructured Ingestion' : '\$_typologyLens Semantic Web',
        'facet': 'Locally Signed & Hashed',
        'origin': 'Native Desktop Edge',
        'region': '0xAA...BB',
        'magnet': 'magnet:?xt=urn:btih:NEW...',
        'alpTokenId': 'ALP-NEW-1',
        'isGhost': false
      };
    });
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        children: [
          // Telemetry
          Row(
            children: [
              _buildTelemetryCard('Active Seeders', '42 Nodes', Icons.wifi_tethering, const Color(0xFF00F0FF)),
              const SizedBox(width: 16),
              _buildTelemetryCard('Swarm Tx Speed', '12.4 MB/s', Icons.share, const Color(0xFF00FF88)),
              const SizedBox(width: 16),
              _buildTelemetryCard('Vectorized Assets', '${_assets.length} Indexed', Icons.tag, const Color(0xFFB026FF)),
            ],
          ),
          const SizedBox(height: 24),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Left Panel (Ingest, Graph, Search)
                Expanded(
                  flex: 1,
                  child: Column(
                    children: [
                      // Ingest Box
                      _buildGlassContainer(
                        child: Padding(
                          padding: const EdgeInsets.all(16.0),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                children: [
                                  const Text('Semantic Typology Lens', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 12, fontFamily: 'monospace')),
                                  DropdownButton<String>(
                                    value: _typologyLens,
                                    dropdownColor: Colors.black87,
                                    style: const TextStyle(color: Color(0xFFFF9900), fontSize: 12, fontWeight: FontWeight.bold),
                                    underline: const SizedBox(),
                                    items: ['Generic', 'Meme', 'Heraldry'].map((v) => DropdownMenuItem(value: v, child: Text(v))).toList(),
                                    onChanged: (v) => setState(() => _typologyLens = v!),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 16),
                              GestureDetector(
                                onTap: _handleIngest,
                                child: Container(
                                  padding: const EdgeInsets.all(24),
                                  decoration: BoxDecoration(
                                    border: Border.all(color: _pipelineState == 'idle' ? const Color(0xFFFF9900).withOpacity(0.3) : const Color(0xFFFF9900), width: 2),
                                    borderRadius: BorderRadius.circular(12),
                                    color: _pipelineState == 'idle' ? Colors.transparent : const Color(0xFFFF9900).withOpacity(0.1),
                                  ),
                                  child: Center(
                                    child: Column(
                                      children: [
                                        Icon(_pipelineState == 'idle' ? Icons.upload_file : Icons.sync, color: const Color(0xFFFF9900), size: 32),
                                        const SizedBox(height: 8),
                                        Text(
                                          _pipelineState == 'idle' ? 'Ingest Multimodal Asset' : 'Native Swarm: LLaVA Extraction in Progress...',
                                          textAlign: TextAlign.center,
                                          style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 12),
                                        ),
                                      ],
                                    ),
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // 2D Swarm Topology
                      Expanded(
                        child: _buildGlassContainer(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Padding(
                                padding: EdgeInsets.all(16.0),
                                child: Text('2D Swarm Topology', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                              ),
                              Expanded(
                                child: Center(
                                  child: AnimatedBuilder(
                                    animation: _spinnerController,
                                    builder: (context, child) {
                                      return Stack(
                                        alignment: Alignment.center,
                                        children: [
                                          Transform.rotate(
                                            angle: _spinnerController.value * 2 * math.pi,
                                            child: Container(
                                              width: 160, height: 160,
                                              decoration: BoxDecoration(
                                                shape: BoxShape.circle,
                                                border: Border.all(color: const Color(0xFF00F0FF).withOpacity(0.5), width: 2),
                                              ),
                                            ),
                                          ),
                                          Transform.rotate(
                                            angle: -(_spinnerController.value * 2 * math.pi),
                                            child: Container(
                                              width: 100, height: 100,
                                              decoration: BoxDecoration(
                                                shape: BoxShape.circle,
                                                border: Border.all(color: const Color(0xFFB026FF).withOpacity(0.5), width: 2),
                                              ),
                                            ),
                                          ),
                                          const Icon(Icons.circle, color: Color(0xFF00FF88), size: 16),
                                        ],
                                      );
                                    }
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // Search Matrix
                      _buildGlassContainer(
                        child: Padding(
                          padding: const EdgeInsets.all(16.0),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Text('Faceted Query (SPARQL-MM)', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                              const SizedBox(height: 12),
                              TextField(
                                decoration: InputDecoration(
                                  filled: true,
                                  fillColor: Colors.black45,
                                  hintText: 'Region, Timecode, Semantics...',
                                  hintStyle: const TextStyle(color: Colors.grey, fontSize: 12, fontFamily: 'monospace'),
                                  prefixIcon: const Icon(Icons.search, color: Colors.grey, size: 20),
                                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                                  contentPadding: EdgeInsets.zero,
                                ),
                                style: const TextStyle(color: Colors.white, fontSize: 12, fontFamily: 'monospace'),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 24),
                // Right Panel (Library)
                Expanded(
                  flex: 2,
                  child: _buildGlassContainer(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Padding(
                          padding: EdgeInsets.all(16.0),
                          child: Text('Multimodal Library', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                        ),
                        Expanded(
                          child: ListView.builder(
                            padding: const EdgeInsets.symmetric(horizontal: 16.0),
                            itemCount: _assets.length,
                            itemBuilder: (context, index) {
                              final asset = _assets[index];
                              final isGhost = asset['isGhost'];
                              
                              return Container(
                                margin: const EdgeInsets.only(bottom: 12),
                                padding: const EdgeInsets.all(12),
                                decoration: BoxDecoration(
                                  color: isGhost ? const Color(0xFFFF9900).withOpacity(0.05) : Colors.black.withOpacity(0.3),
                                  borderRadius: BorderRadius.circular(8),
                                  border: Border.all(color: isGhost ? const Color(0xFFFF9900).withOpacity(0.3) : Colors.white10),
                                ),
                                child: Row(
                                  children: [
                                    Container(
                                      width: 48, height: 48,
                                      decoration: BoxDecoration(
                                        color: Colors.white.withOpacity(0.05),
                                        borderRadius: BorderRadius.circular(8),
                                        border: Border.all(color: Colors.white10),
                                      ),
                                      child: Icon(Icons.image, color: isGhost ? const Color(0xFFFF9900) : Colors.grey),
                                    ),
                                    const SizedBox(width: 16),
                                    Expanded(
                                      child: Column(
                                        crossAxisAlignment: CrossAxisAlignment.start,
                                        children: [
                                          Row(
                                            children: [
                                              Text(asset['id'], style: TextStyle(color: isGhost ? const Color(0xFFFF9900) : Colors.white, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                                              if (!isGhost) ...[
                                                const SizedBox(width: 8),
                                                const Icon(Icons.verified_user, color: Color(0xFF00FF88), size: 14),
                                              ]
                                            ],
                                          ),
                                          const SizedBox(height: 4),
                                          Wrap(
                                            spacing: 8,
                                            children: [
                                              Text(asset['type'], style: const TextStyle(color: Color(0xFFB026FF), fontSize: 10, fontFamily: 'monospace')),
                                              Text('Facet: ${asset['facet']}', style: const TextStyle(color: Color(0xFF00F0FF), fontSize: 10, fontFamily: 'monospace')),
                                              Text('Region: ${asset['region']}', style: const TextStyle(color: Color(0xFFFF9900), fontSize: 10, fontFamily: 'monospace')),
                                            ],
                                          ),
                                        ],
                                      ),
                                    ),
                                    if (!isGhost)
                                      Column(
                                        crossAxisAlignment: CrossAxisAlignment.end,
                                        children: [
                                          Text(asset['magnet'], style: const TextStyle(color: Colors.grey, fontSize: 10, fontFamily: 'monospace')),
                                          const SizedBox(height: 8),
                                          Row(
                                            children: [
                                              if (asset['alpTokenId'] == null)
                                                ElevatedButton(
                                                  onPressed: () {},
                                                  style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFFFFD700).withOpacity(0.1), foregroundColor: const Color(0xFFFFD700), minimumSize: Size.zero, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4)),
                                                  child: const Text('MINT ALP', style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold)),
                                                )
                                              else
                                                Container(
                                                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                                                  decoration: BoxDecoration(border: Border.all(color: const Color(0xFF00FF88).withOpacity(0.5)), borderRadius: BorderRadius.circular(4)),
                                                  child: Text(asset['alpTokenId'], style: const TextStyle(color: Color(0xFF00FF88), fontSize: 10, fontFamily: 'monospace')),
                                                ),
                                              const SizedBox(width: 8),
                                              ElevatedButton(
                                                onPressed: () {},
                                                style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF00F0FF).withOpacity(0.1), foregroundColor: const Color(0xFF00F0FF), minimumSize: Size.zero, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4)),
                                                child: const Text('P2P FETCH', style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold)),
                                              ),
                                            ],
                                          ),
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
          ),
        ],
      ),
    );
  }

  Widget _buildTelemetryCard(String title, String val, IconData icon, Color color) {
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
                  Text(val, style: const TextStyle(color: Colors.white, fontSize: 20, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
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
