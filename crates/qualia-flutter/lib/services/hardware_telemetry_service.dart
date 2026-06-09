import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/qualia_api.dart' as api;
import '../tray/tray_service.dart';

/// Live CPU/RAM/VRAM/daemon stats polled every 2 seconds.
class HardwareTelemetrySnapshot {
  const HardwareTelemetrySnapshot({
    required this.engine,
    this.vramAvailableGb,
  });

  final api.HardwareTelemetry engine;
  final double? vramAvailableGb;
}

class HardwareTelemetryNotifier extends StateNotifier<HardwareTelemetrySnapshot?> {
  HardwareTelemetryNotifier() : super(null);

  Timer? _timer;

  void start() {
    _timer?.cancel();
    poll();
    _timer = Timer.periodic(const Duration(seconds: 2), (_) => poll());
  }

  Future<void> poll() async {
    try {
      final telemetry = await api.getHardwareTelemetry();
      double? vramGb;
      try {
        final status = await api.getHardwareStatus();
        if (status.vramEstimatedGb > 0) {
          vramGb = status.vramEstimatedGb;
        }
      } catch (_) {}
      final snapshot = HardwareTelemetrySnapshot(
        engine: telemetry,
        vramAvailableGb: vramGb,
      );
      state = snapshot;
      await TrayService.instance.updateFromTelemetry(telemetry);
    } catch (_) {}
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}

final hardwareTelemetryProvider =
    StateNotifierProvider<HardwareTelemetryNotifier, HardwareTelemetrySnapshot?>(
  (ref) {
    final notifier = HardwareTelemetryNotifier();
    notifier.start();
    ref.onDispose(notifier.dispose);
    return notifier;
  },
);
