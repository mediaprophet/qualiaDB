import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

import '../tray/tray_service.dart';

bool get isDesktopTarget {
  if (kIsWeb) return false;
  return Platform.isWindows || Platform.isLinux || Platform.isMacOS;
}

/// Initializes window_manager + tray for desktop; no-op on web/mobile.
Future<void> initDesktopShell() async {
  if (!isDesktopTarget) return;

  await windowManager.ensureInitialized();
  await windowManager.setPreventClose(true);

  const options = WindowOptions(
    size: Size(1200, 800),
    minimumSize: Size(900, 600),
    center: true,
    title: 'QualiaDB',
  );

  windowManager.waitUntilReadyToShow(options, () async {
    await windowManager.show();
    await windowManager.focus();
  });

  await TrayService.instance.init();
}
