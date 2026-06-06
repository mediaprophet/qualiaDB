import 'package:flutter/material.dart';

class DashboardScreen extends StatefulWidget {
  const DashboardScreen({super.key});

  @override
  State<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends State<DashboardScreen> {
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
    
    // Simulate rust execution
    await Future.delayed(const Duration(seconds: 2));
    _appendLog('Command completed successfully.');
    
    setState(() => _runningTask = null);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
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
          const SizedBox(height: 24),
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
                label: Text(_runningTask == 'zk_screen' ? 'Running...' : '[Zero-Knowledge] Toxicity Screening'),
                onPressed: _runningTask != null ? null : () => _runCommand('zk_screen', '[Zero-Knowledge] Toxicity Screening'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: const Color(0xFF00F0FF).withOpacity(0.2),
                  foregroundColor: const Color(0xFF00F0FF),
                  padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),
          Expanded(
            child: Container(
              decoration: BoxDecoration(
                color: Colors.black,
                border: Border.all(color: const Color(0xFF00F0FF).withOpacity(0.3)),
                borderRadius: BorderRadius.circular(8.0),
              ),
              padding: const EdgeInsets.all(16.0),
              child: ListView.builder(
                itemCount: _logs.length,
                itemBuilder: (context, index) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4.0),
                    child: Text(
                      _logs[index],
                      style: const TextStyle(
                        fontFamily: 'monospace',
                        color: Colors.greenAccent,
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        ],
      ),
    );
  }
}
