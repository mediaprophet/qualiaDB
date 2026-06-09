import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/hardware_telemetry_service.dart';
import '../services/pending_affirmations_service.dart';
import '../src/rust/api/qualia_api.dart' as api;

/// Mechanical sympathy HUD: RAM floor, thermal state, model lifecycle, daemon.
class VaultHudBar extends ConsumerWidget {
  final bool dense;

  const VaultHudBar({super.key, this.dense = false});

  static const _amberMb = 384;
  static const _criticalMb = 512;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final snapshot = ref.watch(hardwareTelemetryProvider);
    final telemetry = snapshot?.engine;
    final vramGb = snapshot?.vramAvailableGb;
    final pendingCount = ref.watch(pendingAffirmationCountProvider);
    final cs = Theme.of(context).colorScheme;

    final llmMb = _llmMemoryMb(telemetry?.llmMemoryBytes);
    final ramPressure = _ramPressureLevel(llmMb, telemetry?.memoryFloorMb ?? _criticalMb);
    final thermal = telemetry?.thermalState ?? 'Cool';
    final lifecycle = telemetry?.modelLifecycle ?? 'Discovered';
    final isScrubbing = lifecycle == 'Scrubbing';
    final isCriticalThermal = thermal == 'Critical';

    final barColor = isCriticalThermal
        ? cs.errorContainer.withValues(alpha: dense ? 0.35 : 0.5)
        : ramPressure == _RamPressure.critical
            ? cs.errorContainer.withValues(alpha: 0.4)
            : ramPressure == _RamPressure.amber
                ? Colors.amber.withValues(alpha: 0.12)
                : Colors.black.withValues(alpha: dense ? 0.2 : 0.4);

    return Container(
      width: double.infinity,
      padding: EdgeInsets.symmetric(
        horizontal: dense ? 10 : 16,
        vertical: dense ? 4 : 10,
      ),
      decoration: BoxDecoration(
        color: barColor,
        border: Border(
          bottom: BorderSide(color: cs.outlineVariant.withValues(alpha: 0.35)),
        ),
      ),
      child: dense
          ? _denseRow(context, telemetry, vramGb, llmMb, ramPressure, thermal, lifecycle, isScrubbing, pendingCount, ref)
          : _fullRow(context, telemetry, vramGb, llmMb, ramPressure, thermal, lifecycle, isScrubbing, pendingCount, ref),
    );
  }

  Widget _fullRow(
    BuildContext context,
    api.HardwareTelemetry? telemetry,
    double? vramGb,
    double llmMb,
    _RamPressure ramPressure,
    String thermal,
    String lifecycle,
    bool isScrubbing,
    int pendingCount,
    WidgetRef ref,
  ) {
    return Row(
      children: [
        const Text('Vault:', style: TextStyle(color: Colors.grey, fontSize: 12)),
        const SizedBox(width: 6),
        _lifecycleChip(lifecycle, isScrubbing),
        const SizedBox(width: 16),
        _metric(Icons.memory, 'CPU ${telemetry?.cpuPercent.toStringAsFixed(1) ?? '—'}%'),
        const SizedBox(width: 12),
        _ramSegment(llmMb, ramPressure, telemetry?.memoryFloorMb ?? _criticalMb),
        const SizedBox(width: 12),
        _thermalBadge(thermal),
        const SizedBox(width: 12),
        _daemonDot(telemetry?.daemonStatus ?? '…'),
        const SizedBox(width: 12),
        _pendingAffirmationsButton(context, pendingCount, ref),
        const Spacer(),
        if (telemetry != null && telemetry.kvCacheUsedMb > 0)
          Text(
            'KV ${telemetry.kvCacheUsedMb} MB',
            style: const TextStyle(color: Colors.grey, fontSize: 11, fontFamily: 'monospace'),
          ),
        if (telemetry != null && telemetry.vramTotalMb > 0) ...[
          const SizedBox(width: 12),
          Text(
            'VRAM ${telemetry.vramUsedMb}/${telemetry.vramTotalMb} MB',
            style: const TextStyle(color: Colors.grey, fontSize: 11, fontFamily: 'monospace'),
          ),
        ] else if (vramGb != null && vramGb > 0) ...[
          const SizedBox(width: 12),
          Text(
            'VRAM ${vramGb.toStringAsFixed(1)} GB free',
            style: const TextStyle(color: Colors.grey, fontSize: 11, fontFamily: 'monospace'),
          ),
        ],
      ],
    );
  }

  Widget _denseRow(
    BuildContext context,
    api.HardwareTelemetry? telemetry,
    double? vramGb,
    double llmMb,
    _RamPressure ramPressure,
    String thermal,
    String lifecycle,
    bool isScrubbing,
    int pendingCount,
    WidgetRef ref,
  ) {
    return Row(
      children: [
        _lifecycleChip(lifecycle, isScrubbing, small: true),
        const SizedBox(width: 8),
        Expanded(child: _ramSegment(llmMb, ramPressure, telemetry?.memoryFloorMb ?? _criticalMb, small: true)),
        if (vramGb != null && vramGb > 0) ...[
          const SizedBox(width: 8),
          Text(
            telemetry != null && telemetry.vramTotalMb > 0
                ? 'VRAM ${telemetry.vramUsedMb}/${telemetry.vramTotalMb}M'
                : 'VRAM ${vramGb.toStringAsFixed(0)}G',
            style: const TextStyle(color: Colors.grey, fontSize: 10, fontFamily: 'monospace'),
          ),
        ],
        const SizedBox(width: 8),
        _thermalBadge(thermal, small: true),
        const SizedBox(width: 8),
        _daemonDot(telemetry?.daemonStatus ?? '…', small: true),
        if (pendingCount > 0) ...[
          const SizedBox(width: 8),
          _pendingAffirmationsButton(context, pendingCount, ref, small: true),
        ],
      ],
    );
  }

  Widget _pendingAffirmationsButton(
    BuildContext context,
    int count,
    WidgetRef ref, {
    bool small = false,
  }) {
    return Badge(
      isLabelVisible: count > 0,
      label: Text('$count'),
      backgroundColor: Colors.amber,
      child: IconButton(
        visualDensity: VisualDensity.compact,
        iconSize: small ? 16 : 20,
        tooltip: count > 0 ? '$count pending affirmation(s)' : 'Pending affirmations',
        onPressed: () {
          ref.read(showPendingPanelProvider.notifier).state = true;
        },
        icon: Icon(
          Icons.family_restroom,
          size: small ? 16 : 20,
          color: count > 0 ? Colors.amber : Colors.grey,
        ),
      ),
    );
  }

  Widget _metric(IconData icon, String label) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 14, color: const Color(0xFFFF9900)),
        const SizedBox(width: 4),
        Text(
          label,
          style: const TextStyle(color: Colors.white, fontSize: 12, fontFamily: 'monospace'),
        ),
      ],
    );
  }

  Widget _ramSegment(double llmMb, _RamPressure level, int floorMb, {bool small = false}) {
    final color = switch (level) {
      _RamPressure.critical => const Color(0xFFFF4444),
      _RamPressure.amber => Colors.amber,
      _RamPressure.ok => const Color(0xFF00FF88),
    };
    final label = level == _RamPressure.critical
        ? 'Memory ceiling (${llmMb.toStringAsFixed(0)}/$floorMb MB)'
        : 'LLM ${llmMb.toStringAsFixed(0)} MB';

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(Icons.storage, size: small ? 12 : 14, color: color),
        const SizedBox(width: 4),
        Flexible(
          child: Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: small ? 10 : 12,
              fontFamily: 'monospace',
            ),
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }

  Widget _thermalBadge(String thermal, {bool small = false}) {
    final (color, label) = switch (thermal) {
      'Critical' => (const Color(0xFFFF4444), 'Cooling: Logic Throttled'),
      'Warm' => (Colors.amber, 'Logic may throttle'),
      _ => (const Color(0xFF00F0FF), 'Thermal $thermal'),
    };
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(Icons.thermostat, size: small ? 12 : 14, color: color),
        const SizedBox(width: 4),
        Text(
          label,
          style: TextStyle(color: color, fontSize: small ? 10 : 11, fontFamily: 'monospace'),
        ),
      ],
    );
  }

  Widget _lifecycleChip(String lifecycle, bool isScrubbing, {bool small = false}) {
    final color = isScrubbing ? Colors.orange : const Color(0xFF00F0FF);
    return Container(
      padding: EdgeInsets.symmetric(horizontal: small ? 6 : 8, vertical: 2),
      decoration: BoxDecoration(
        border: Border.all(color: color.withValues(alpha: 0.6)),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        lifecycle,
        style: TextStyle(
          color: color,
          fontSize: small ? 10 : 11,
          fontWeight: FontWeight.w600,
          fontFamily: 'monospace',
        ),
      ),
    );
  }

  Widget _daemonDot(String status, {bool small = false}) {
    final live = status == 'running';
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.circle,
          size: small ? 6 : 8,
          color: live ? const Color(0xFF00FF88) : Colors.grey,
        ),
        const SizedBox(width: 4),
        Text(
          small ? (live ? '4242' : 'off') : 'Daemon: $status',
          style: TextStyle(
            color: Colors.grey,
            fontSize: small ? 10 : 12,
            fontFamily: 'monospace',
          ),
        ),
      ],
    );
  }

  _RamPressure _ramPressureLevel(double llmMb, int floorMb) {
    if (llmMb >= floorMb) return _RamPressure.critical;
    if (llmMb >= _amberMb) return _RamPressure.amber;
    return _RamPressure.ok;
  }

  double _llmMemoryMb(BigInt? bytes) {
    if (bytes == null) return 0;
    return bytes.toDouble() / (1024 * 1024);
  }
}

enum _RamPressure { ok, amber, critical }
