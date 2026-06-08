import 'dart:io';

import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

import '../src/rust/api/qualia_api.dart' as api;

/// System tray icon, menu, and show/hide/quit actions for desktop targets.
class TrayService {
  TrayService._();
  static final TrayService instance = TrayService._();

  /// Called when the user picks Settings from the tray menu.
  void Function()? onOpenSettings;

  Future<void> init() async {
    await trayManager.setIcon(await _resolveTrayIconPath());
    await _refreshTooltip();

    final menu = Menu(
      items: [
        MenuItem(key: 'open', label: 'Open QualiaDB'),
        MenuItem(key: 'settings', label: 'Settings'),
        MenuItem.separator(),
        MenuItem(key: 'quit', label: 'Quit'),
      ],
    );
    await trayManager.setContextMenu(menu);
  }

  Future<void> _refreshTooltip() async {
    try {
      final status = await api.daemonStatus();
      await trayManager.setToolTip('QualiaDB — daemon: $status');
    } catch (_) {
      await trayManager.setToolTip('QualiaDB');
    }
  }

  int _pendingAffirmations = 0;

  Future<void> updatePendingCount(int count) async {
    _pendingAffirmations = count;
  }

  Future<void> updateFromTelemetry(api.HardwareTelemetry telemetry) async {
    final llmMb =
        (telemetry.llmMemoryBytes.toDouble() / (1024 * 1024)).toStringAsFixed(0);
    final pending = _pendingAffirmations > 0 ? ' | Pending: $_pendingAffirmations' : '';
    await trayManager.setToolTip(
      'QualiaDB — CPU ${telemetry.cpuPercent.toStringAsFixed(0)}% | '
      'RAM ${telemetry.ramUsedGb.toStringAsFixed(1)} GB | '
      'LLM $llmMb MB | ${telemetry.thermalState} | '
      '${telemetry.modelLifecycle} | daemon: ${telemetry.daemonStatus}$pending',
    );
  }

  Future<String> _resolveTrayIconPath() async {
    final asset = Platform.isWindows
        ? 'assets/icons/tray_icon.ico'
        : 'assets/icons/tray_icon.png';
    final bytes = (await rootBundle.load(asset)).buffer.asUint8List();
    final dir = await getTemporaryDirectory();
    final ext = Platform.isWindows ? 'ico' : 'png';
    final file = File('${dir.path}/qualia_tray.$ext');
    await file.writeAsBytes(bytes, flush: true);
    return file.path;
  }

  Future<void> showMainWindow() async {
    await windowManager.show();
    await windowManager.focus();
  }

  Future<void> hideMainWindow() async {
    await windowManager.hide();
  }

  Future<void> quitApp() async {
    await trayManager.destroy();
    await windowManager.destroy();
  }

  Future<void> handleMenuClick(MenuItem item) async {
    switch (item.key) {
      case 'open':
        await showMainWindow();
      case 'settings':
        onOpenSettings?.call();
        await showMainWindow();
      case 'quit':
        await quitApp();
      default:
        break;
    }
  }

  /// Left-click / primary tray activation — toggle visibility.
  Future<void> handleTrayIconActivated() async {
    if (await windowManager.isVisible()) {
      await hideMainWindow();
    } else {
      await showMainWindow();
    }
  }

  /// Right-click — show context menu (Windows/Linux).
  Future<void> showContextMenu() async {
    await trayManager.popUpContextMenu();
  }
}
