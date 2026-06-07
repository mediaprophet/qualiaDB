import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'screens/dashboard_screen.dart';
import 'screens/chat_screen.dart';
import 'screens/wallet_screen.dart';
import 'screens/address_book_screen.dart';
import 'screens/settings_screen.dart';

import 'screens/ontology_hub_screen.dart';
import 'screens/asset_library_screen.dart';
import 'screens/app_vault_screen.dart';
import 'screens/credential_manager_screen.dart';
import 'screens/llm_hub_screen.dart';
import 'screens/spatial_physics_screen.dart';

import 'src/rust/api/qualia_api.dart';
import 'src/rust/frb_generated.dart';

/// The absolute path to the currently active `.gguf` model file.
/// Empty string means no model is selected.
/// Written by LLMHubScreen (file picker), read by ChatScreen (inference).
final activeModelPathProvider = StateProvider<String>((ref) => '');

Future<void> main() async {
  await RustApi.init();
  await initCore();
  runApp(const ProviderScope(child: QualiaApp()));
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

class _QualiaHomeScreenState extends ConsumerState<QualiaHomeScreen> {
  int _currentIndex = 0;

  /// Build the screen widget for [index], injecting live state where needed.
  Widget _buildScreen(int index) {
    switch (index) {
      case 1:
        // Chat screen reads the active model path from the provider.
        return ChatScreen(modelPath: ref.watch(activeModelPathProvider));
      default:
        return _staticScreens[index];
    }
  }

  /// Screens that need no dynamic state can stay as const widgets.
  static const List<Widget> _staticScreens = [
    DashboardScreen(),       // 0
    SizedBox.shrink(),       // 1 — replaced by _buildScreen (ChatScreen)
    WalletScreen(),          // 2
    AddressBookScreen(),     // 3
    OntologyHubScreen(),     // 4
    AssetLibraryScreen(),    // 5
    AppVaultScreen(),        // 6
    CredentialManagerScreen(), // 7
    LLMHubScreen(),          // 8
    SpatialPhysicsScreen(),  // 9
    SettingsScreen(),        // 10
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Row(
        children: [
          // Navigation Sidebar
          Container(
            width: 80,
            color: Theme.of(context).colorScheme.surface,
            child: SingleChildScrollView(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
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
                  const SizedBox(height: 16),
                ],
              ),
            ),
          ),
          // Main Content Area
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
