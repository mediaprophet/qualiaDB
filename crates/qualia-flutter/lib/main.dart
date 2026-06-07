import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:tray_manager/tray_manager.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:window_manager/window_manager.dart';

import 'screens/dashboard_screen.dart';
import 'screens/chat_screen.dart';
import 'screens/wallet_screen.dart';
import 'screens/address_book_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/profile_screen.dart';
import 'screens/ontology_hub_screen.dart';
import 'screens/asset_library_screen.dart';
import 'screens/qapp_vault_screen.dart';
import 'screens/credential_manager_screen.dart';
import 'screens/llm_hub_screen.dart';
import 'screens/spatial_physics_screen.dart';
import 'screens/qualia_qapp_webview.dart';

import 'platform/desktop_window.dart';
import 'screens/prerequisites_overlay.dart';
import 'screens/setup_wizard_screen.dart';
import 'services/deep_link_service.dart';
import 'services/update_checker.dart';

import 'src/rust/api/qualia_api.dart';
import 'src/rust/frb_generated.dart';
import 'tray/tray_service.dart';
import 'widgets/hardware_telemetry_bar.dart';

/// The absolute path to the currently active `.gguf` model file.
final activeModelPathProvider = StateProvider<String>((ref) => '');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  _installDesktopErrorHandlers();
  await RustApi.init();
  await initCore();
  await initDesktopShell();
  runApp(const ProviderScope(child: QualiaApp()));
}

void _installDesktopErrorHandlers() {
  if (!isDesktopTarget) return;

  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    debugPrint('FlutterError: ${details.exceptionAsString()}');
    if (details.stack != null) debugPrint(details.stack.toString());
  };

  PlatformDispatcher.instance.onError = (error, stack) {
    debugPrint('Uncaught async error: $error\n$stack');
    return false;
  };
}

class QualiaApp extends StatelessWidget {
  const QualiaApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'QualiaDB',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF0D0D12),
          brightness: Brightness.dark,
          primary: const Color(0xFF5D5FEF),
          secondary: const Color(0xFF1BEBB9),
          surface: const Color(0xFF141419),
        ),
        useMaterial3: true,
      ),
      home: const QualiaHomeScreen(),
    );
  }
}

class QualiaHomeScreen extends ConsumerStatefulWidget {
  const QualiaHomeScreen({super.key});

  @override
  ConsumerState<QualiaHomeScreen> createState() => _QualiaHomeScreenState();
}

class _QualiaHomeScreenState extends ConsumerState<QualiaHomeScreen>
    with WindowListener, TrayListener {
  int _currentIndex = 0;
  bool _showPrerequisites = false;
  bool _showSetup = false;

  @override
  void initState() {
    super.initState();
    if (isDesktopTarget) {
      windowManager.addListener(this);
      trayManager.addListener(this);
      TrayService.instance.onOpenSettings = () {
        if (mounted) setState(() => _currentIndex = 10);
      };
    }
    _checkStartup();
    _initDeepLinks();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) UpdateChecker.checkAndNotify(context);
    });
  }

  Future<void> _initDeepLinks() async {
    if (isDesktopTarget) {
      try {
        if (Platform.isWindows) {
          await registerQualiaUriHandler(exePath: Platform.resolvedExecutable);
        }
      } catch (e, st) {
        debugPrint('qualia:// handler registration failed: $e\n$st');
      }
    }
    DeepLinkService.instance.listen(_handleDeepLink);
    final initial = await DeepLinkService.instance.getInitialLink();
    if (initial != null && mounted) _handleDeepLink(initial);
  }

  void _handleDeepLink(QualiaDeepLink link) {
    switch (link.route) {
      case 'settings':
        setState(() => _currentIndex = 10);
      case 'chat':
        setState(() => _currentIndex = 1);
      case 'wallet':
        setState(() => _currentIndex = 2);
      case 'qapp':
        if (link.qappName != null) _openQualiaQapp(link.qappName!);
    }
  }

  Future<void> _openQualiaQapp(String qappName) async {
    try {
      final url = await launchInstalledQapp(qappName: qappName);
      if (!mounted) return;
      if (url.startsWith('http://127.0.0.1') || url.startsWith('http://localhost')) {
        await Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => QualiaQappWebView(url: url, title: qappName),
          ),
        );
      } else {
        final uri = Uri.parse(url);
        if (!await launchUrl(uri, mode: LaunchMode.externalApplication)) {
          throw Exception('Could not open $url');
        }
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Qapp launch failed: $e')),
        );
      }
    }
  }

  Future<void> _checkStartup() async {
    if (Platform.isWindows) {
      try {
        final status = await checkPrerequisites();
        if (mounted && !status.allReady) {
          setState(() => _showPrerequisites = true);
          return;
        }
      } catch (e, st) {
        debugPrint('Prerequisite check failed: $e\n$st');
      }
    }
    await _checkFirstRun();
  }

  Future<void> _checkFirstRun() async {
    try {
      final first = await isFirstRun();
      if (mounted) setState(() => _showSetup = first);
    } catch (e, st) {
      debugPrint('First-run check failed: $e\n$st');
    }
  }

  @override
  void dispose() {
    if (isDesktopTarget) {
      windowManager.removeListener(this);
      trayManager.removeListener(this);
      TrayService.instance.onOpenSettings = null;
    }
    DeepLinkService.instance.dispose();
    super.dispose();
  }

  @override
  void onWindowClose() {
    // Minimize to tray instead of exiting (daemon keeps running).
    TrayService.instance.hideMainWindow();
  }

  @override
  void onTrayIconMouseDown() {
    TrayService.instance.handleTrayIconActivated();
  }

  @override
  void onTrayIconRightMouseDown() {
    TrayService.instance.showContextMenu();
  }

  @override
  void onTrayMenuItemClick(MenuItem menuItem) {
    TrayService.instance.handleMenuClick(menuItem);
  }

  Widget _buildScreen(int index) {
    switch (index) {
      case 1:
        return ChatScreen(modelPath: ref.watch(activeModelPathProvider));
      default:
        return _staticScreens[index];
    }
  }

  static const List<Widget> _staticScreens = [
    DashboardScreen(),
    SizedBox.shrink(),
    WalletScreen(),
    AddressBookScreen(),
    OntologyHubScreen(),
    AssetLibraryScreen(),
    QappVaultScreen(),
    CredentialManagerScreen(),
    LLMHubScreen(),
    SpatialPhysicsScreen(),
    SettingsScreen(),
    ProfileScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        Column(
          children: [
            if (isDesktopTarget) const HardwareTelemetryBar(),
            Expanded(child: _buildShell()),
          ],
        ),
        if (_showPrerequisites)
          PrerequisitesOverlay(
            onComplete: () {
              setState(() => _showPrerequisites = false);
              _checkFirstRun();
            },
          ),
        if (_showSetup)
          SetupWizardOverlay(onComplete: () => setState(() => _showSetup = false)),
      ],
    );
  }

  Widget _buildShell() {
    return Scaffold(
      body: Row(
        children: [
          Container(
            width: 80,
            color: Theme.of(context).colorScheme.surface,
            child: SingleChildScrollView(
              child: Column(
                children: [
                  const SizedBox(height: 16),
                  _buildNavItem(Icons.dashboard_outlined, 0),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.chat_bubble_outline, 1),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.account_balance_wallet_outlined, 2),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.perm_contact_calendar_outlined, 3),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.hub_outlined, 4),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.photo_library_outlined, 5),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.apps_outlined, 6),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.key_outlined, 7),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.memory_outlined, 8),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.view_in_ar_outlined, 9),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.settings_outlined, 10),
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.person_outline, 11),
                  const SizedBox(height: 16),
                ],
              ),
            ),
          ),
          Expanded(
            child: ColoredBox(
              color: Theme.of(context).colorScheme.surface,
              child: _buildScreen(_currentIndex),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildNavItem(IconData icon, int index) {
    final isActive = _currentIndex == index;
    return IconButton(
      icon: Icon(icon),
      onPressed: () => setState(() => _currentIndex = index),
      color: isActive
          ? Theme.of(context).colorScheme.primary
          : Theme.of(context).colorScheme.onSurface.withOpacity(0.5),
    );
  }
}
