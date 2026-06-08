import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/qualia_api.dart' as api;
import '../tray/tray_service.dart';

/// Polls the CRDT suspended-transaction queue for bilateral guardianship UI.
class PendingAffirmationsNotifier extends StateNotifier<List<api.SuspendedTxView>> {
  PendingAffirmationsNotifier() : super(const []);

  Timer? _timer;

  void start() {
    _timer?.cancel();
    poll();
    _timer = Timer.periodic(const Duration(seconds: 3), (_) => poll());
  }

  Future<void> poll() async {
    try {
      final pending = await api.listPendingAffirmations();
      state = pending;
      await TrayService.instance.updatePendingCount(pending.length);
    } catch (_) {}
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}

final pendingAffirmationsProvider =
    StateNotifierProvider<PendingAffirmationsNotifier, List<api.SuspendedTxView>>(
  (ref) {
    final notifier = PendingAffirmationsNotifier();
    notifier.start();
    ref.onDispose(notifier.dispose);
    return notifier;
  },
);

final pendingAffirmationCountProvider = Provider<int>((ref) {
  return ref.watch(pendingAffirmationsProvider).length;
});

final showPendingPanelProvider = StateProvider<bool>((ref) => false);
