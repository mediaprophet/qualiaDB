import 'dart:async';

import '../src/rust/api/qualia_api.dart' as api;

/// Waits for a background model activation to finish, polling Rust state and
/// optional [LLM_LOAD|active|] / [LLM_LOAD|failed|] telemetry lines.
Future<void> waitForModelActivation({
  Duration pollInterval = const Duration(milliseconds: 200),
  Duration timeout = const Duration(minutes: 10),
}) async {
  final deadline = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(deadline)) {
    final err = await api.takeModelActivationError();
    if (err != null && err.isNotEmpty) {
      throw Exception(err);
    }
    final inProgress = await api.isModelActivationInProgress();
    if (!inProgress) {
      final lateErr = await api.takeModelActivationError();
      if (lateErr != null && lateErr.isNotEmpty) {
        throw Exception(lateErr);
      }
      return;
    }
    await Future<void>.delayed(pollInterval);
  }
  throw TimeoutException('Model activation timed out', timeout);
}

/// Starts async activation and waits for completion without blocking the FRB thread.
Future<void> activateModelAsync(String modelName) async {
  await api.setActiveModelAsync(modelName: modelName);
  await waitForModelActivation();
}
