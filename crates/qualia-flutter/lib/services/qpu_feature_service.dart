import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/qualia_api.dart' as api;

/// Whether the hidden QPU Oracle settings panel is visible.
final qpuFeatureUnlockedProvider = StateNotifierProvider<QpuFeatureNotifier, bool>(
  (ref) => QpuFeatureNotifier(),
);

class QpuFeatureNotifier extends StateNotifier<bool> {
  QpuFeatureNotifier() : super(false) {
    _load();
  }

  Future<void> _load() async {
    try {
      state = await api.isQpuFeatureUnlocked();
    } catch (_) {
      state = false;
    }
  }

  Future<void> refresh() => _load();

  void setUnlocked(bool value) => state = value;
}
