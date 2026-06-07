import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/hardware_telemetry_service.dart';

/// Status strip matching the Tauri React app header (CPU, RAM, daemon).
class HardwareTelemetryBar extends ConsumerWidget {
  const HardwareTelemetryBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final telemetry = ref.watch(hardwareTelemetryProvider);

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.4),
        border: Border(bottom: BorderSide(color: Colors.white.withValues(alpha: 0.08))),
      ),
      child: Row(
        children: [
          const Text('Engine:', style: TextStyle(color: Colors.grey, fontSize: 12)),
          const SizedBox(width: 6),
          const Text('NATIVE', style: TextStyle(color: Color(0xFF00F0FF), fontSize: 12, fontWeight: FontWeight.bold)),
          const SizedBox(width: 24),
          const Icon(Icons.memory, size: 14, color: Color(0xFFFF9900)),
          const SizedBox(width: 4),
          Text(
            'CPU: ${telemetry?.cpuPercent.toStringAsFixed(1) ?? '—'}%',
            style: const TextStyle(color: Colors.white, fontSize: 12, fontFamily: 'monospace'),
          ),
          const SizedBox(width: 16),
          const Icon(Icons.storage, size: 14, color: Color(0xFF00FF88)),
          const SizedBox(width: 4),
          Text(
            telemetry != null
                ? 'RAM: ${telemetry.ramUsedGb.toStringAsFixed(2)} GB'
                : 'RAM: —',
            style: const TextStyle(color: Colors.white, fontSize: 12, fontFamily: 'monospace'),
          ),
          const SizedBox(width: 16),
          Icon(
            Icons.circle,
            size: 8,
            color: telemetry?.daemonStatus == 'running' ? const Color(0xFF00FF88) : Colors.grey,
          ),
          const SizedBox(width: 6),
          Text(
            'Daemon: ${telemetry?.daemonStatus ?? '…'}',
            style: const TextStyle(color: Colors.grey, fontSize: 12, fontFamily: 'monospace'),
          ),
          const Spacer(),
          const Text(
            'DAG: mediaprophet/qualiaDB',
            style: TextStyle(color: Color(0xFFB026FF), fontSize: 11, fontFamily: 'monospace'),
          ),
        ],
      ),
    );
  }
}
