import 'dart:ui';
import 'package:flutter/material.dart';
import 'dart:math' as math;
import '../src/rust/api/qualia_api.dart';

class OntologyHubScreen extends StatefulWidget {
  const OntologyHubScreen({super.key});

  @override
  State<OntologyHubScreen> createState() => _OntologyHubScreenState();
}

class _OntologyHubScreenState extends State<OntologyHubScreen> with TickerProviderStateMixin {
  late AnimationController _pulseController;
  late AnimationController _rotationController;
  int _selectedNode = -1;
  List<CatalogItem> _ontologies = [];

  final List<Map<String, dynamic>> _nodes = [
    {'id': 0, 'label': 'Subject', 'x': 0.2, 'y': 0.5},
    {'id': 1, 'label': 'Predicate (skos:exactMatch)', 'x': 0.5, 'y': 0.3},
    {'id': 2, 'label': 'Object (Dense Tensor)', 'x': 0.8, 'y': 0.5},
    {'id': 3, 'label': 'Inferred Entity', 'x': 0.5, 'y': 0.7},
  ];

  final List<Map<String, dynamic>> _edges = [
    {'source': 0, 'target': 1, 'type': 'explicit'},
    {'source': 1, 'target': 2, 'type': 'explicit'},
    {'source': 0, 'target': 3, 'type': 'inferred'},
  ];

  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(vsync: this, duration: const Duration(seconds: 2))..repeat(reverse: true);
    _rotationController = AnimationController(vsync: this, duration: const Duration(seconds: 20))..repeat();
    _loadOntologies();
  }

  Future<void> _loadOntologies() async {
    final list = await fetchOntologyCatalogReal();
    if (mounted) {
      setState(() => _ontologies = list);
    }
  }

  @override
  void dispose() {
    _pulseController.dispose();
    _rotationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        children: [
          // Telemetry Row
          Row(
            children: [
              _buildTelemetryCard('Total Lexicon Nodes', '14,023', Icons.bolt, const Color(0xFF00FF88)),
              const SizedBox(width: 16),
              _buildTelemetryCard('Active Defeasible Claims', '892', Icons.show_chart, const Color(0xFFFFD700)),
              const SizedBox(width: 16),
              _buildTelemetryCard('Ontologies Indexed', '0', Icons.memory, const Color(0xFF00F0FF)),
            ],
          ),
          const SizedBox(height: 24),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Vector Space Canvas
                Expanded(
                  flex: 2,
                  child: _buildGlassContainer(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Padding(
                          padding: EdgeInsets.all(16.0),
                          child: Text('Vector Space Canvas (.q42.bidx)', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.white, fontFamily: 'monospace')),
                        ),
                        Expanded(
                          child: Stack(
                            children: [
                              // Background grid
                              Positioned.fill(
                                child: CustomPaint(painter: GridPainter()),
                              ),
                              // Rotating Background Glow
                              Center(
                                child: AnimatedBuilder(
                                  animation: _rotationController,
                                  builder: (context, child) {
                                    return Transform.rotate(
                                      angle: _rotationController.value * 2 * math.pi,
                                      child: Container(
                                        width: 300,
                                        height: 300,
                                        decoration: BoxDecoration(
                                          shape: BoxShape.circle,
                                          gradient: RadialGradient(
                                            colors: [
                                              const Color(0xFF00F0FF).withOpacity(0.1),
                                              Colors.transparent,
                                            ],
                                          ),
                                        ),
                                      ),
                                    );
                                  }
                                ),
                              ),
                              // Interactive Nodes and Edges
                              LayoutBuilder(
                                builder: (context, constraints) {
                                  return CustomPaint(
                                    size: Size(constraints.maxWidth, constraints.maxHeight),
                                    painter: GraphPainter(
                                      nodes: _nodes,
                                      edges: _edges,
                                      selectedIndex: _selectedNode,
                                      pulseValue: _pulseController.value,
                                    ),
                                  );
                                },
                              ),
                              // Invisible touch targets for nodes
                              LayoutBuilder(
                                builder: (context, constraints) {
                                  return Stack(
                                    children: _nodes.map((node) {
                                      final dx = node['x'] * constraints.maxWidth;
                                      final dy = node['y'] * constraints.maxHeight;
                                      return Positioned(
                                        left: dx - 24,
                                        top: dy - 24,
                                        width: 48,
                                        height: 48,
                                        child: GestureDetector(
                                          onTap: () => setState(() => _selectedNode = node['id']),
                                          child: Container(color: Colors.transparent),
                                        ),
                                      );
                                    }).toList(),
                                  );
                                },
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 24),
                // Right Panel (Inspector + List)
                Expanded(
                  flex: 1,
                  child: Column(
                    children: [
                      // Node Inspector
                      _buildGlassContainer(
                        child: Padding(
                          padding: const EdgeInsets.all(16.0),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Text('Node Inspector', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.white, fontFamily: 'monospace')),
                              const SizedBox(height: 16),
                              if (_selectedNode == -1)
                                const Center(
                                  child: Text('Select a node on the canvas.', style: TextStyle(color: Colors.grey, fontFamily: 'monospace', fontSize: 12)),
                                )
                              else
                                Column(
                                  children: [
                                    _buildInspectorRow('Identifier', '0x${_selectedNode}A144E6', const Color(0xFF00F0FF)),
                                    _buildInspectorRow('Label', _nodes[_selectedNode]['label'], Colors.white),
                                    _buildInspectorRow('State', 'Explicitly Indexed', const Color(0xFF00FF88)),
                                  ],
                                ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 24),
                      Container(
                        padding: const EdgeInsets.all(16),
                        decoration: BoxDecoration(color: Colors.black45, borderRadius: BorderRadius.circular(12), border: Border.all(color: Colors.white10)),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('Search Nodes', style: TextStyle(color: Colors.grey, fontSize: 12)),
                            const SizedBox(height: 8),
                            TextField(
                              style: const TextStyle(color: Colors.white, fontSize: 14),
                              decoration: InputDecoration(
                                hintText: 'e.g. dopamine_receptor',
                                hintStyle: const TextStyle(color: Colors.white30),
                                prefixIcon: const Icon(Icons.search, color: Colors.white54, size: 16),
                                isDense: true,
                                filled: true,
                                fillColor: Colors.white.withOpacity(0.05),
                                border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      Expanded(
                        child: _buildGlassContainer(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Padding(
                                padding: EdgeInsets.all(16.0),
                                child: Text('Global Ontologies', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.white, fontFamily: 'monospace')),
                              ),
                              Expanded(
                                child: _ontologies.isEmpty 
                                ? const Center(child: CircularProgressIndicator(color: Color(0xFF00F0FF)))
                                : ListView.builder(
                                  padding: const EdgeInsets.symmetric(horizontal: 16.0),
                                  itemCount: _ontologies.length,
                                  itemBuilder: (context, index) {
                                    final o = _ontologies[index];
                                    return Container(
                                      margin: const EdgeInsets.only(bottom: 12),
                                      padding: const EdgeInsets.all(12),
                                      decoration: BoxDecoration(
                                        color: Colors.black.withOpacity(0.3),
                                        borderRadius: BorderRadius.circular(8),
                                        border: Border.all(color: Colors.white10),
                                      ),
                                      child: Column(
                                        crossAxisAlignment: CrossAxisAlignment.start,
                                        children: [
                                          Row(
                                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                            children: [
                                              Expanded(child: Text(o.name, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13), overflow: TextOverflow.ellipsis)),
                                              Text(o.size, style: const TextStyle(color: Colors.grey, fontSize: 10, fontFamily: 'monospace')),
                                            ],
                                          ),
                                          const SizedBox(height: 8),
                                          OutlinedButton.icon(
                                            onPressed: () {},
                                            icon: const Icon(Icons.download, size: 14),
                                            label: const Text('Download & Index', style: TextStyle(fontSize: 11)),
                                            style: OutlinedButton.styleFrom(
                                              foregroundColor: const Color(0xFF00F0FF),
                                              side: const BorderSide(color: Color(0xFF00F0FF)),
                                              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                                            ),
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
                  Text(val, style: const TextStyle(color: Colors.white, fontSize: 24, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildInspectorRow(String attr, String val, Color valColor) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(attr, style: const TextStyle(color: Colors.grey, fontSize: 12, fontFamily: 'monospace')),
          Text(val, style: TextStyle(color: valColor, fontSize: 12, fontFamily: 'monospace')),
        ],
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

class GridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white.withOpacity(0.02)
      ..strokeWidth = 1.0;
    for (double i = 0; i < size.width; i += 40) {
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), paint);
    }
    for (double i = 0; i < size.height; i += 40) {
      canvas.drawLine(Offset(0, i), Offset(size.width, i), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class GraphPainter extends CustomPainter {
  final List<Map<String, dynamic>> nodes;
  final List<Map<String, dynamic>> edges;
  final int selectedIndex;
  final double pulseValue;

  GraphPainter({required this.nodes, required this.edges, required this.selectedIndex, required this.pulseValue});

  @override
  void paint(Canvas canvas, Size size) {
    // Draw edges
    for (final edge in edges) {
      final s = nodes[edge['source']];
      final t = nodes[edge['target']];
      final p1 = Offset(s['x'] * size.width, s['y'] * size.height);
      final p2 = Offset(t['x'] * size.width, t['y'] * size.height);

      final paint = Paint()
        ..color = edge['type'] == 'explicit' ? const Color(0xFF00F0FF) : const Color(0xFFFFD700)
        ..strokeWidth = 2.0
        ..style = PaintingStyle.stroke;

      if (edge['type'] == 'inferred') {
        _drawDashedLine(canvas, p1, p2, paint);
      } else {
        canvas.drawLine(p1, p2, paint);
        // Add glow for explicit
        canvas.drawLine(p1, p2, paint..strokeWidth = 6.0..color = const Color(0xFF00F0FF).withOpacity(0.3));
      }
    }

    // Draw nodes
    for (final node in nodes) {
      final p = Offset(node['x'] * size.width, node['y'] * size.height);
      final isSelected = node['id'] == selectedIndex;

      // Pulse effect
      if (isSelected) {
        canvas.drawCircle(p, 12 + (pulseValue * 8), Paint()..color = const Color(0xFFB026FF).withOpacity(0.3));
      }

      final fill = Paint()..color = isSelected ? const Color(0xFFB026FF) : const Color(0xFF1A1A2E);
      final stroke = Paint()
        ..color = isSelected ? Colors.white : const Color(0xFF00F0FF)
        ..strokeWidth = 3.0
        ..style = PaintingStyle.stroke;

      canvas.drawCircle(p, 12, fill);
      canvas.drawCircle(p, 12, stroke);

      // Add label
      final textSpan = TextSpan(
        text: node['label'],
        style: const TextStyle(color: Colors.white, fontSize: 10, fontFamily: 'monospace'),
      );
      final textPainter = TextPainter(text: textSpan, textDirection: TextDirection.ltr);
      textPainter.layout();
      textPainter.paint(canvas, Offset(p.dx - (textPainter.width / 2), p.dy - 30));
    }
  }

  void _drawDashedLine(Canvas canvas, Offset p1, Offset p2, Paint paint) {
    const dashWidth = 5.0;
    const dashSpace = 5.0;
    double distance = (p2 - p1).distance;
    double dx = (p2.dx - p1.dx) / distance;
    double dy = (p2.dy - p1.dy) / distance;
    
    double start = 0;
    while (start < distance) {
      canvas.drawLine(
        Offset(p1.dx + dx * start, p1.dy + dy * start),
        Offset(p1.dx + dx * (start + dashWidth), p1.dy + dy * (start + dashWidth)),
        paint,
      );
      start += dashWidth + dashSpace;
    }
  }

  @override
  bool shouldRepaint(covariant GraphPainter oldDelegate) => true; // Always repaint for animations
}
