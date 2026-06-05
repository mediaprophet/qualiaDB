import 'dart:ui';
import 'package:flutter/material.dart';
import '../src/rust/api/qualia_api.dart';

class AppStoreScreen extends StatefulWidget {
  const AppStoreScreen({super.key});

  @override
  State<AppStoreScreen> createState() => _AppStoreScreenState();
}

class _AppStoreScreenState extends State<AppStoreScreen> {
  final List<Map<String, String>> _apps = [];
  String _vcInput = '';
  String _generatedVc = '';
  String _launchingId = '';
  String _installStatus = '';

  @override
  void initState() {
    super.initState();
    // Simulate fetching apps from rust
    Future.delayed(const Duration(milliseconds: 500), () {
      if (mounted) {
        setState(() {
          _apps.addAll([
            {'id': 'com.qualia.explorer', 'name': 'Graph Explorer', 'status': 'Installed', 'vc': 'Valid'},
            {'id': 'com.qualia.mesh', 'name': 'Mesh Monitor', 'status': 'Installed', 'vc': 'Valid'},
          ]);
        });
      }
    });
  }

  void _handleLaunch(String appId) {
    setState(() => _launchingId = appId);
    Future.delayed(const Duration(seconds: 2), () {
      if (mounted) setState(() => _launchingId = '');
    });
  }

  Future<void> _handleInstallApp() async {
    setState(() => _installStatus = 'Installing...');
    try {
      final res = await verifyAndInstallApp(zipPath: 'dummy/path.zip', credentialSig: 'did:qualia:app:123');
      if (mounted) {
        setState(() => _installStatus = res);
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(res)));
      }
    } catch (e) {
      if (mounted) {
        setState(() => _installStatus = 'Error: $e');
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Install Failed: $e')));
      }
    }
  }

  void _handleSignVc() {
    if (_vcInput.isEmpty) return;
    setState(() {
      _generatedVc = 'vc:qualia:test_signature_for_$_vcInput';
    });
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Apps Section
          _buildGlassContainer(
            child: Padding(
              padding: const EdgeInsets.all(24.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Row(
                        children: [
                          const Icon(Icons.apps, color: Color(0xFF00F0FF), size: 28),
                          const SizedBox(width: 12),
                          const Text('Local App Manager', style: TextStyle(color: Colors.white, fontSize: 20, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                        ],
                      ),
                      ElevatedButton.icon(
                        onPressed: _handleInstallApp,
                        icon: const Icon(Icons.add_box, size: 16),
                        label: const Text('Install Package'),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: const Color(0xFF00F0FF).withOpacity(0.1),
                          foregroundColor: const Color(0xFF00F0FF),
                          side: BorderSide(color: const Color(0xFF00F0FF).withOpacity(0.3)),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  const Text('Install and manage third-party edge-native web apps. Apps are sandboxed and verified via VCs.', style: TextStyle(color: Colors.grey, fontSize: 12)),
                  const SizedBox(height: 24),
                  if (_apps.isEmpty)
                    const Center(
                      child: Padding(
                        padding: EdgeInsets.symmetric(vertical: 32.0),
                        child: Text('No apps installed — place app directories in your Apps/ data folder.', style: TextStyle(color: Colors.grey, fontFamily: 'monospace', fontSize: 12)),
                      ),
                    )
                  else
                    Column(
                      children: _apps.map((app) {
                        final isLaunching = _launchingId == app['id'];
                        return Container(
                          margin: const EdgeInsets.only(bottom: 16),
                          padding: const EdgeInsets.all(16),
                          decoration: BoxDecoration(
                            color: Colors.black.withOpacity(0.4),
                            borderRadius: BorderRadius.circular(12),
                            border: Border.all(color: Colors.white.withOpacity(0.05)),
                          ),
                          child: Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(app['name']!, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16)),
                                  const SizedBox(height: 8),
                                  Row(
                                    children: [
                                      const Icon(Icons.shield, color: Color(0xFF00FF88), size: 12),
                                      const SizedBox(width: 4),
                                      Text('VC: ${app['vc']}', style: const TextStyle(color: Color(0xFF00FF88), fontFamily: 'monospace', fontSize: 10)),
                                      const SizedBox(width: 16),
                                      Text('ID: ${app['id']}', style: const TextStyle(color: Colors.grey, fontFamily: 'monospace', fontSize: 10)),
                                    ],
                                  ),
                                ],
                              ),
                              ElevatedButton.icon(
                                onPressed: isLaunching ? null : () => _handleLaunch(app['id']!),
                                icon: Icon(isLaunching ? Icons.sync : Icons.play_arrow, size: 16),
                                label: Text(isLaunching ? 'Launching...' : 'Launch'),
                                style: ElevatedButton.styleFrom(
                                  backgroundColor: Colors.white.withOpacity(0.1),
                                  foregroundColor: Colors.white,
                                ),
                              ),
                            ],
                          ),
                        );
                      }).toList(),
                    ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),
          // Credentials Section
          _buildGlassContainer(
            child: Container(
              decoration: BoxDecoration(
                border: Border.all(color: const Color(0xFFFFD700).withOpacity(0.3)),
                borderRadius: BorderRadius.circular(16),
              ),
              padding: const EdgeInsets.all(24.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.key, color: Color(0xFFFFD700), size: 28),
                      const SizedBox(width: 12),
                      const Text('Developer Credentials', style: TextStyle(color: Colors.white, fontSize: 20, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                    ],
                  ),
                  const SizedBox(height: 8),
                  const Text('Generate Verifiable Credentials (VCs) to self-sign your own local applications before loading them into the daemon.', style: TextStyle(color: Colors.grey, fontSize: 12)),
                  const SizedBox(height: 24),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          onChanged: (v) => _vcInput = v,
                          style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 14),
                          decoration: InputDecoration(
                            hintText: 'App ID (e.g. com.my.app)',
                            hintStyle: const TextStyle(color: Colors.grey),
                            filled: true,
                            fillColor: Colors.black.withOpacity(0.5),
                            border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                          ),
                        ),
                      ),
                      const SizedBox(width: 16),
                      ElevatedButton(
                        onPressed: _handleSignVc,
                        style: ElevatedButton.styleFrom(
                          backgroundColor: const Color(0xFFFFD700).withOpacity(0.1),
                          foregroundColor: const Color(0xFFFFD700),
                          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
                        ),
                        child: const Text('Sign & Generate VC', style: TextStyle(fontWeight: FontWeight.bold)),
                      ),
                    ],
                  ),
                  if (_generatedVc.isNotEmpty) ...[
                    const SizedBox(height: 16),
                    Container(
                      padding: const EdgeInsets.all(16),
                      decoration: BoxDecoration(
                        color: const Color(0xFF00FF88).withOpacity(0.05),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: const Color(0xFF00FF88).withOpacity(0.2)),
                      ),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(child: Text(_generatedVc, style: const TextStyle(color: Color(0xFF00FF88), fontFamily: 'monospace', fontSize: 12))),
                          IconButton(
                            icon: const Icon(Icons.copy, color: Color(0xFF00FF88), size: 16),
                            onPressed: () {}, // copy stub
                          ),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildGlassContainer({required Widget child}) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white.withOpacity(0.03),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: Colors.white.withOpacity(0.1)),
          ),
          child: child,
        ),
      ),
    );
  }
}
