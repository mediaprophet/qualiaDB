import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'screens/dashboard_screen.dart';
import 'screens/chat_screen.dart';
import 'screens/wallet_screen.dart';
import 'screens/address_book_screen.dart';
import 'screens/settings_screen.dart';

import 'screens/ontology_hub_screen.dart';
import 'screens/asset_library_screen.dart';
import 'screens/app_store_screen.dart';
import 'screens/credential_manager_screen.dart';
import 'screens/llm_hub_screen.dart';
import 'screens/spatial_physics_screen.dart';

// Import the generated rust bridge bindings
import 'src/rust/api/qualia_api.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
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

class QualiaHomeScreen extends StatefulWidget {
  const QualiaHomeScreen({super.key});

  @override
  State<QualiaHomeScreen> createState() => _QualiaHomeScreenState();
}

class _QualiaHomeScreenState extends State<QualiaHomeScreen> {
  int _currentIndex = 0;

  final List<Widget> _screens = [
    const DashboardScreen(),
    const ChatScreen(),
    const WalletScreen(),
    const AddressBookScreen(),
    const OntologyHubScreen(),
    const AssetLibraryScreen(),
    const AppStoreScreen(),
    const CredentialManagerScreen(),
    const LLMHubScreen(),
    const SpatialPhysicsScreen(),
    const SettingsScreen(),
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
                  _buildNavItem(Icons.hub_outlined, 4), // Ontology Hub
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.photo_library_outlined, 5), // Asset Library
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.apps_outlined, 6), // App Store
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.key_outlined, 7), // Credential Manager
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.memory_outlined, 8), // LLM Hub
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.view_in_ar_outlined, 9), // Spatial Physics
                  const SizedBox(height: 32),
                  _buildNavItem(Icons.settings_outlined, 10), // Settings
                  const SizedBox(height: 16),
                ],
              ),
            ),
          ),
          // Main Content Area
          Expanded(
            child: Container(
              color: Theme.of(context).colorScheme.background,
              child: _screens[_currentIndex],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildNavItem(IconData icon, int index) {
    return IconButton(
      icon: Icon(icon),
      onPressed: () => setState(() => _currentIndex = index),
      color: _currentIndex == index 
          ? Theme.of(context).colorScheme.primary 
          : Theme.of(context).colorScheme.onSurface.withOpacity(0.5),
    );
  }
}
