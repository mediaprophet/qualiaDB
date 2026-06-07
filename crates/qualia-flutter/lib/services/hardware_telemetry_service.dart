import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/qualia_api.dart' as api;
import '../tray/tray_service.dart';

/// Live CPU/RAM/daemon stats polled every 2 seconds (matches Tauri telemetry loop).
class HardwareTelemetryNotifier extends StateNotifier<api.HardwareTelemetry?> {
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
      state = telemetry;
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
    StateNotifierProvider<HardwareTelemetryNotifier, api.HardwareTelemetry?>(
  (ref) {
    final notifier = HardwareTelemetryNotifier();
    notifier.start();
    ref.onDispose(notifier.dispose);
    return notifier;
  },
);
