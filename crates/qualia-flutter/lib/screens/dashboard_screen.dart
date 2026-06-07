import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/hardware_telemetry_service.dart';
import '../src/rust/api/qualia_api.dart' as api;

class DashboardScreen extends ConsumerStatefulWidget {
  const DashboardScreen({super.key});

  @override
  ConsumerState<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends ConsumerState<DashboardScreen> {
  final List<String> _logs = ['[System] Initialized. Awaiting ILP routes.'];
  String? _runningTask;

  void _appendLog(String message) {
    setState(() {
      _logs.add('[${DateTime.now().toIso8601String().substring(11, 19)}] $message');
    });
  }

  Future<void> _runCommand(String cmd, String label) async {
    if (_runningTask != null) return;
    setState(() => _runningTask = cmd);
    _appendLog('> Executing: $label');
    try {
      final result = await api.runEngineCommand(cmd: cmd);
      _appendLog(result.replaceAll('\n', ' | '));
    } catch (e) {
      _appendLog('Error: $e');
    } finally {
      setState(() => _runningTask = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final telemetry = ref.watch(hardwareTelemetryProvider);    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Edge-Native Benchmarks',
            style: Theme.of(context).textTheme.headlineMedium?.copyWith(
              color: Theme.of(context).colorScheme.primary,
              fontWeight: FontWeight.bold,
            ),
          ),
          if (telemetry != null) ...[
            const SizedBox(height: 8),
            Text(
              'CPU ${telemetry.cpuPercent.toStringAsFixed(1)}% · '
              'RAM ${telemetry.ramUsedGb.toStringAsFixed(2)} / ${telemetry.ramTotalGb.toStringAsFixed(2)} GB · '
              'daemon ${telemetry.daemonStatus}',
              style: const TextStyle(color: Colors.grey, fontSize: 12, fontFamily: 'monospace'),
            ),
          ],          const SizedBox(height: 24),
          Row(
            children: [
              ElevatedButton.icon(
                icon: const Icon(Icons.rocket_launch),
                label: Text(_runningTask == 'ingest_bench' ? 'Running...' : 'Ingest 100,000 Quins'),
                onPressed: _runningTask != null ? null : () => _runCommand('ingest_bench', 'Ingest 100,000 Quins'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: const Color(0xFFFFD700).withOpacity(0.2),
                  foregroundColor: const Color(0xFFFFD700),
                  padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                ),
              ),
              const SizedBox(width: 16),
              ElevatedButton.icon(
                icon: const Icon(Icons.security),
                label: Text(_runningTask == 'zk_screen' ? 'Running...' : 'Zero-Knowledge Screen'),
                onPressed: _runningTask != null ? null : () => _runCommand('zk_screen', 'Zero-Knowledge Screen'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: const Color(0xFF00F0FF).withOpacity(0.2),
                  foregroundColor: const Color(0xFF00F0FF),
                  padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                ),
              ),
            ],
          ),
          const SizedBox(height: 32),
          Expanded(
            child: Container(
              width: double.infinity,
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.black,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white10),
              ),
              child: ListView.builder(
                itemCount: _logs.length,
                itemBuilder: (context, i) => Text(
                  _logs[i],
                  style: const TextStyle(fontFamily: 'monospace', fontSize: 12, color: Color(0xFF00FF88)),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
