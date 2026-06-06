import 'dart:ui';
import 'dart:async';
import 'package:flutter/material.dart';
import 'dart:math' as math;
import '../src/rust/api/qualia_api.dart';

class SpatialPhysicsScreen extends StatefulWidget {
  const SpatialPhysicsScreen({super.key});

  @override
  State<SpatialPhysicsScreen> createState() => _SpatialPhysicsScreenState();
}

class _SpatialPhysicsScreenState extends State<SpatialPhysicsScreen> with TickerProviderStateMixin {
  double _temperature = 50.0;
  double _pressure = 50.0;
  double _timeDilation = 1.0;
  String _daemonStatus = 'stopped';

  late AnimationController _engineController;
  Timer? _stateSyncTimer;

  @override
  void initState() {
    super.initState();
    _engineController = AnimationController(vsync: this, duration: const Duration(seconds: 10))..repeat();
    _loadState();
    
    // Sync UI state down to rust occasionally if it drifts, or just on change.
    // We'll update the Rust backend synchronously when sliders move, and occasionally sync back.
    _stateSyncTimer = Timer.periodic(const Duration(seconds: 2), (_) => _loadState());
  }

  Future<void> _loadState() async {
    final state = await getPhysicsState();
    final daemon = await daemonStatus();
    if (mounted) {
      setState(() {
        _temperature = state.temperature;
        _pressure = state.pressure;
        _timeDilation = state.timeDilation;
        _daemonStatus = daemon;
      });
    }
  }

  Future<void> _handleStartDaemon() async {
    await startDaemon();
    _loadState();
  }

  Future<void> _updateState(double temp, double press, double timeDil) async {
    setState(() {
      _temperature = temp;
      _pressure = press;
      _timeDilation = timeDil;
    });
    await updatePhysicsState(temperature: temp, pressure: press, timeDilation: timeDil);
  }

  @override
  void dispose() {
    _stateSyncTimer?.cancel();
    _engineController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Adjust animation speed based on time dilation
    _engineController.duration = Duration(milliseconds: (10000 / math.max(0.1, _timeDilation)).round());
    if (!_engineController.isAnimating) _engineController.repeat();

    final isCritical = _temperature > 70;
    final primaryColor = isCritical ? const Color(0xFFFF4444) : const Color(0xFF00F0FF);

    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Telemetry
          Row(
            children: [
              _buildTelemetryCard('Active Payload', '.q42_SuperBlock_A1', Icons.layers, const Color(0xFF00F0FF)),
              const SizedBox(width: 16),
              _buildTelemetryCard('Thermodynamic State', isCritical ? 'CRITICAL' : 'STABLE', Icons.thermostat, primaryColor, borderActive: isCritical),
            ],
          ),
          const SizedBox(height: 24),
          
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Left Panel (Controls)
                Expanded(
                  flex: 1,
                  child: _buildGlassContainer(
                    child: Padding(
                      padding: const EdgeInsets.all(24.0),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          const Row(
                            children: [
                              Icon(Icons.tune, color: Color(0xFF00F0FF), size: 24),
                              SizedBox(width: 12),
                              Text('Physics Engine Variables', style: TextStyle(color: Colors.white, fontSize: 16, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                            ],
                          ),
                          const SizedBox(height: 32),
                          
                          _buildSlider('Ambient Temperature (K)', _temperature, 0, 100, const Color(0xFFFF4444), (v) => _updateState(v, _pressure, _timeDilation), '${(_temperature * 10).toInt()}K'),
                          const SizedBox(height: 32),
                          _buildSlider('Manifold Pressure (hPa)', _pressure, 0, 100, const Color(0xFFB026FF), (v) => _updateState(_temperature, v, _timeDilation), '${(_pressure * 20).toInt()} hPa'),
                          const SizedBox(height: 32),
                          _buildSlider('Temporal Velocity (dt)', _timeDilation, 0, 5, const Color(0xFF00FF88), (v) => _updateState(_temperature, _pressure, v), 'x${_timeDilation.toStringAsFixed(2)}'),
                          
                          const Spacer(),
                          Container(
                            padding: const EdgeInsets.all(16),
                            decoration: BoxDecoration(color: Colors.black45, borderRadius: BorderRadius.circular(8), border: Border.all(color: Colors.white10)),
                            child: Row(
                              children: [
                                Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    const Row(
                                      children: [
                                        Icon(Icons.waves, color: Color(0xFF00F0FF), size: 16),
                                        SizedBox(width: 8),
                                        Text('Daemon Status', style: TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                                      ],
                                    ),
                                    const SizedBox(height: 8),
                                    Text(_daemonStatus == 'running' ? 'Active (:4242)' : 'Stopped', style: TextStyle(color: _daemonStatus == 'running' ? const Color(0xFF00FF88) : Colors.grey, fontSize: 10, fontFamily: 'monospace')),
                                  ],
                                ),
                                const Spacer(),
                                ElevatedButton(
                                  onPressed: _daemonStatus == 'running' ? null : _handleStartDaemon,
                                  style: ElevatedButton.styleFrom(
                                    backgroundColor: _daemonStatus == 'running' ? Colors.transparent : const Color(0xFF00FF88).withOpacity(0.1),
                                    foregroundColor: const Color(0xFF00FF88),
                                    side: BorderSide(color: _daemonStatus == 'running' ? Colors.transparent : const Color(0xFF00FF88).withOpacity(0.3)),
                                  ),
                                  child: Text(_daemonStatus == 'running' ? 'Online' : 'Start Core DB'),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 24),
                // Right Panel (Canvas)
                Expanded(
                  flex: 2,
                  child: _buildGlassContainer(
                    child: Stack(
                      children: [
                        const Positioned(
                          top: 24, left: 24,
                          child: Row(
                            children: [
                              Icon(Icons.view_in_ar, color: Color(0xFFB026FF), size: 20),
                              SizedBox(width: 8),
                              Text('Spatial Mesh Renderer', style: TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                            ],
                          ),
                        ),
                        Center(
                          child: AnimatedBuilder(
                            animation: _engineController,
                            builder: (context, child) {
                              return CustomPaint(
                                size: const Size(400, 400),
                                painter: WireframePainter(
                                  progress: _engineController.value,
                                  temperature: _temperature,
                                  pressure: _pressure,
                                  baseColor: primaryColor,
                                ),
                              );
                            }
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

  Widget _buildSlider(String label, double value, double min, double max, Color color, ValueChanged<double> onChanged, String displayValue) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(label, style: const TextStyle(color: Colors.grey, fontSize: 12, fontFamily: 'monospace')),
            Text(displayValue, style: TextStyle(color: color, fontSize: 12, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
          ],
        ),
        const SizedBox(height: 8),
        SliderTheme(
          data: SliderThemeData(
            activeTrackColor: color,
            inactiveTrackColor: color.withOpacity(0.2),
            thumbColor: color,
            overlayColor: color.withOpacity(0.1),
            trackHeight: 2.0,
          ),
          child: Slider(value: value, min: min, max: max, onChanged: onChanged),
        ),
      ],
    );
  }

  Widget _buildTelemetryCard(String title, String val, IconData icon, Color color, {bool borderActive = false}) {
    return Expanded(
      child: _buildGlassContainer(
        borderColor: borderActive ? color.withOpacity(0.5) : Colors.white.withOpacity(0.1),
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

  Widget _buildGlassContainer({required Widget child, Color? borderColor}) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white.withOpacity(0.03),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: borderColor ?? Colors.white.withOpacity(0.1)),
          ),
          child: child,
        ),
      ),
    );
  }
}

class WireframePainter extends CustomPainter {
  final double progress;
  final double temperature;
  final double pressure;
  final Color baseColor;

  WireframePainter({required this.progress, required this.temperature, required this.pressure, required this.baseColor});

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    
    // Calculate warp and scale based on physics
    final scale = 1.0 + (pressure / 100) * 0.5;
    final warp = math.sin(progress * math.pi * 2) * (temperature / 100) * 0.5;
    
    final radius = 100.0 * scale;
    
    // Rotate the 3D-like shape
    final angle = progress * math.pi * 2;
    
    // Draw an isometric octahedron wireframe
    final top = Offset(center.dx, center.dy - radius * (1.5 + warp));
    final bottom = Offset(center.dx, center.dy + radius * (1.5 + warp));
    
    final points = <Offset>[];
    for (int i = 0; i < 4; i++) {
      final a = angle + (i * math.pi / 2);
      // Perspective projection simulation
      final x = math.cos(a) * radius;
      final y = math.sin(a) * (radius * 0.4); // isometric squish
      points.add(Offset(center.dx + x, center.dy + y));
    }

    final paint = Paint()
      ..color = baseColor
      ..strokeWidth = 2.0
      ..style = PaintingStyle.stroke;

    final glowPaint = Paint()
      ..color = baseColor.withOpacity(0.3)
      ..strokeWidth = 6.0
      ..style = PaintingStyle.stroke;

    // Draw equator
    for (int i = 0; i < 4; i++) {
      final p1 = points[i];
      final p2 = points[(i + 1) % 4];
      canvas.drawLine(p1, p2, paint);
      canvas.drawLine(p1, p2, glowPaint);
    }

    // Draw lines to top and bottom
    for (final p in points) {
      canvas.drawLine(p, top, paint);
      canvas.drawLine(p, top, glowPaint);
      
      canvas.drawLine(p, bottom, paint);
      canvas.drawLine(p, bottom, glowPaint);
    }
  }

  @override
  bool shouldRepaint(covariant WireframePainter oldDelegate) => true; // Always repaint for continuous animation
}
