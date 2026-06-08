import 'dart:ui';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'dart:math' as math;
import '../src/rust/api/qualia_api.dart'; // generated bindings

class CredentialManagerScreen extends StatefulWidget {
  const CredentialManagerScreen({super.key});

  @override
  State<CredentialManagerScreen> createState() => _CredentialManagerScreenState();
}

class _CredentialManagerScreenState extends State<CredentialManagerScreen> with TickerProviderStateMixin {
  String _seedPhrase = '';
  bool _identityLoaded = false;
  Map<String, String>? _derivedWallets;
  List<Map<String, String>> _importedAccounts = [];

  // Import Modal State
  bool _showImportModal = false;
  String _importNetwork = 'eCash (XEC)';
  String _importLabel = '';
  String _importSeed = '';

  late AnimationController _glowController;

  @override
  void initState() {
    super.initState();
    _glowController = AnimationController(vsync: this, duration: const Duration(seconds: 2))..repeat(reverse: true);
    _loadInitialState();
  }

  Future<void> _loadInitialState() async {
    try {
      final identityJsonStr = await loadIdentity();
      if (identityJsonStr != null) {
        final Map<String, dynamic> parsed = jsonDecode(identityJsonStr);
        setState(() {
          _identityLoaded = true;
          _derivedWallets = parsed.map((k, v) => MapEntry(k, v.toString()));
        });
      }

      final importedStr = await loadImportedAccounts();
      if (importedStr.isNotEmpty) {
        final List<dynamic> parsedList = jsonDecode(importedStr);
        setState(() {
          _importedAccounts = parsedList.map((e) => Map<String, String>.from(e)).toList();
        });
      }
    } catch (e) {
      debugPrint("Error loading identity: \$e");
    }
  }

  @override
  void dispose() {
    _glowController.dispose();
    super.dispose();
  }

  Future<void> _handleGenerateSeed() async {
    try {
      final seed = await generateBip39Seed();
      final walletsJsonStr = await deriveWalletsFromSeed(seed: seed);
      
      await saveIdentity(walletsJson: walletsJsonStr);

      final Map<String, dynamic> parsed = jsonDecode(walletsJsonStr);

      setState(() {
        _seedPhrase = seed;
        _identityLoaded = true;
        _derivedWallets = parsed.map((k, v) => MapEntry(k, v.toString()));
      });
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Error: \$e')));
    }
  }

  Future<void> _handleImportAccount() async {
    if (_importSeed.isEmpty || _importLabel.isEmpty) return;
    try {
      final address = await importExternalSeed(network: _importNetwork, seed: _importSeed, label: _importLabel);
      
      final Map<String, dynamic> newAccountRaw = {
        'network': _importNetwork,
        'label': _importLabel,
        'address': address,
      };
      final Map<String, String> newAccount = newAccountRaw.map((k, v) => MapEntry(k, v.toString()));

      final updatedAccounts = List<Map<String, String>>.from(_importedAccounts)..add(newAccount);
      await saveImportedAccounts(accountsJson: jsonEncode(updatedAccounts));

      setState(() {
        _importedAccounts = updatedAccounts;
        _showImportModal = false;
        _importSeed = '';
        _importLabel = '';
      });
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Import Failed: \$e')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        SingleChildScrollView(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Row(
                    children: [
                      Icon(Icons.key, color: Color(0xFFB026FF), size: 28),
                      SizedBox(width: 12),
                      Text('Principal Identifiers & Verifiable Claims', style: TextStyle(color: Colors.white, fontSize: 24, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                    ],
                  ),
                  ElevatedButton.icon(
                    onPressed: () => setState(() => _showImportModal = true),
                    icon: const Icon(Icons.add, size: 16),
                    label: const Text('Add External Identifier'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: const Color(0xFF00F0FF).withOpacity(0.1),
                      foregroundColor: const Color(0xFF00F0FF),
                      side: BorderSide(color: const Color(0xFF00F0FF).withOpacity(0.3)),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 24),
              
              // Principal identifier root
              _buildGlassContainer(
                child: Container(
                  decoration: BoxDecoration(border: Border.all(color: const Color(0xFFB026FF).withOpacity(0.3)), borderRadius: BorderRadius.circular(16)),
                  padding: const EdgeInsets.all(24.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('Principal Identifier Root (BIP39 Material)', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
                      const SizedBox(height: 8),
                      const Text('Your 12-word seed phrase is the cryptographic root of your principal identifier material. It deterministically derives your default decentralized identifiers (DIDs) across the Webizen topology while preserving human agency beyond any single identifier.', style: TextStyle(color: Colors.grey, fontSize: 12, fontFamily: 'monospace')),
                      const SizedBox(height: 24),
                      
                      if (_identityLoaded && _seedPhrase.isEmpty)
                        Container(
                          padding: const EdgeInsets.all(16),
                          decoration: BoxDecoration(
                            color: Colors.black45,
                            border: Border.all(color: const Color(0xFF00FF88).withOpacity(0.3)),
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Row(
                            children: [
                              const Icon(Icons.shield, color: Color(0xFF00FF88), size: 24),
                              const SizedBox(width: 12),
                              const Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text('Principal Identifier Active', style: TextStyle(color: Color(0xFF00FF88), fontWeight: FontWeight.bold, fontSize: 14)),
                                    Text('Seed phrase was shown at creation — back it up offline. Derived addresses and identifier material are restored below.', style: TextStyle(color: Colors.grey, fontSize: 10)),
                                  ],
                                ),
                              ),
                              OutlinedButton(
                                onPressed: _handleGenerateSeed,
                                style: OutlinedButton.styleFrom(foregroundColor: Colors.grey, side: const BorderSide(color: Colors.white10)),
                                child: const Text('Regenerate'),
                              ),
                            ],
                          ),
                        )
                      else if (_seedPhrase.isEmpty)
                        ElevatedButton(
                          onPressed: _handleGenerateSeed,
                          style: ElevatedButton.styleFrom(
                            backgroundColor: Colors.white.withOpacity(0.05),
                            foregroundColor: Colors.white,
                            side: const BorderSide(color: Color(0xFF00F0FF)),
                            padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                          ),
                          child: const Text('Generate 12-Word Seed'),
                        )
                      else
                        Container(
                          padding: const EdgeInsets.all(16),
                          decoration: BoxDecoration(
                            color: Colors.black45,
                            border: Border.all(color: Colors.white10),
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Row(
                                children: [
                                  Icon(Icons.lock, color: Color(0xFF00FF88), size: 12),
                                  SizedBox(width: 8),
                                  Text('WRITE THIS DOWN — NOT STORED ON DISK', style: TextStyle(color: Colors.grey, fontSize: 10, fontFamily: 'monospace', letterSpacing: 1.2)),
                                ],
                              ),
                              const SizedBox(height: 12),
                              Container(
                                width: double.infinity,
                                padding: const EdgeInsets.all(12),
                                decoration: BoxDecoration(color: Colors.black, border: Border.all(color: Colors.white10), borderRadius: BorderRadius.circular(4)),
                                child: SelectableText(_seedPhrase, style: const TextStyle(color: Color(0xFF00F0FF), fontSize: 16, fontFamily: 'monospace', letterSpacing: 2.0)),
                              ),
                              const SizedBox(height: 8),
                              const Text('Your derived addresses have been saved. The seed phrase above will not be shown again after you leave this page.', style: TextStyle(color: Colors.orange, fontSize: 10)),
                            ],
                          ),
                        ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 24),
              
              if (_derivedWallets != null)
                GridView.count(
                  crossAxisCount: 2,
                  shrinkWrap: true,
                  physics: const NeverScrollableScrollPhysics(),
                  crossAxisSpacing: 16,
                  mainAxisSpacing: 16,
                  childAspectRatio: 3.5,
                  children: [
                    _buildWalletCard('Qualia Root', _derivedWallets!['qualia_root']!, const Color(0xFF00F0FF)),
                    _buildWalletCard('Bitcoin (BTC)', _derivedWallets!['bitcoin_btc'] ?? '—', const Color(0xFFF7931A)),
                    _buildWalletCard('Monero (XMR)', _derivedWallets!['monero_xmr'] ?? '—', const Color(0xFFFF6600)),
                    _buildWalletCard(
                      'Nym Mixnet (optional)',
                      _derivedWallets!['nym_mixnet']!,
                      const Color(0xFFB026FF),
                      label: 'Enable in Wallet → Optional privacy',
                    ),
                    _buildWalletCard('eCash (XEC)', _derivedWallets!['ecash_xec']!, const Color(0xFF00FF88)),
                    _buildWalletCard('Ethereum (EVM)', _derivedWallets!['ethereum']!, Colors.grey),
                  ],
                ),
                
              if (_importedAccounts.isNotEmpty) ...[
                const SizedBox(height: 24),
                const Text('Imported External Identifiers', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
                const SizedBox(height: 16),
                GridView.count(
                  crossAxisCount: 2,
                  shrinkWrap: true,
                  physics: const NeverScrollableScrollPhysics(),
                  crossAxisSpacing: 16,
                  mainAxisSpacing: 16,
                  childAspectRatio: 3.5,
                  children: _importedAccounts.map((acc) {
                    return _buildWalletCard(acc['network']!, acc['address']!, Colors.blue, label: acc['label']);
                  }).toList(),
                ),
              ],
            ],
          ),
        ),
        
        // Import Modal
        if (_showImportModal)
          Container(
            color: Colors.black.withOpacity(0.8),
            child: BackdropFilter(
              filter: ImageFilter.blur(sigmaX: 5, sigmaY: 5),
              child: Center(
                child: Container(
                  width: 500,
                  decoration: BoxDecoration(
                    color: const Color(0xFF0A0A0F),
                    borderRadius: BorderRadius.circular(16),
                    border: Border.all(color: const Color(0xFF00F0FF).withOpacity(0.3)),
                    boxShadow: [BoxShadow(color: const Color(0xFF00F0FF).withOpacity(0.1), blurRadius: 50)],
                  ),
                  padding: const EdgeInsets.all(32),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Row(
                        children: [
                          Icon(Icons.download, color: Color(0xFF00F0FF)),
                          SizedBox(width: 12),
                          Text('Import External Identifier Material', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
                        ],
                      ),
                      const SizedBox(height: 24),
                      const Text('NETWORK', style: TextStyle(color: Colors.grey, fontSize: 10, fontWeight: FontWeight.bold, letterSpacing: 1.2)),
                      const SizedBox(height: 8),
                      DropdownButtonFormField<String>(
                        value: _importNetwork,
                        dropdownColor: Colors.black87,
                        decoration: const InputDecoration(filled: true, fillColor: Colors.black45, border: OutlineInputBorder()),
                        style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 12),
                        items: ['eCash (XEC)', 'Bitcoin (BTC)', 'Nym (NYM) - Nyx Chain', 'Monero (XMR)', 'Ethereum (EVM)']
                            .map((e) => DropdownMenuItem(value: e, child: Text(e))).toList(),
                        onChanged: (v) => setState(() => _importNetwork = v!),
                      ),
                      const SizedBox(height: 16),
                      const Text('ACCOUNT LABEL', style: TextStyle(color: Colors.grey, fontSize: 10, fontWeight: FontWeight.bold, letterSpacing: 1.2)),
                      const SizedBox(height: 8),
                      TextField(
                        onChanged: (v) => _importLabel = v,
                        style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 12),
                        decoration: const InputDecoration(filled: true, fillColor: Colors.black45, border: OutlineInputBorder(), hintText: 'e.g. Trading Wallet, Cold Storage...', hintStyle: TextStyle(color: Colors.white24)),
                      ),
                      const SizedBox(height: 16),
                      const Text('12/24 WORD SEED PHRASE', style: TextStyle(color: Colors.grey, fontSize: 10, fontWeight: FontWeight.bold, letterSpacing: 1.2)),
                      const SizedBox(height: 8),
                      TextField(
                        onChanged: (v) => _importSeed = v,
                        maxLines: 3,
                        style: const TextStyle(color: Color(0xFF00FF88), fontFamily: 'monospace', fontSize: 12),
                        decoration: const InputDecoration(filled: true, fillColor: Colors.black45, border: OutlineInputBorder(), hintText: 'abandon ability able about...', hintStyle: TextStyle(color: Colors.white24)),
                      ),
                      const SizedBox(height: 4),
                      const Align(alignment: Alignment.centerRight, child: Text('Secured in OS Keychain upon import', style: TextStyle(color: Colors.grey, fontSize: 8, letterSpacing: 1.2))),
                      const SizedBox(height: 24),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.end,
                        children: [
                          TextButton(
                            onPressed: () => setState(() => _showImportModal = false),
                            child: const Text('Cancel', style: TextStyle(color: Colors.grey)),
                          ),
                          const SizedBox(width: 16),
                          ElevatedButton(
                            onPressed: _handleImportAccount,
                            style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF00F0FF).withOpacity(0.2), foregroundColor: const Color(0xFF00F0FF)),
                            child: const Text('Import Identifier'),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }

  Widget _buildWalletCard(String title, String address, Color themeColor, {String? label}) {
    return _buildGlassContainer(
      child: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(begin: Alignment.topLeft, end: Alignment.bottomRight, colors: [Colors.black, themeColor.withOpacity(0.1)]),
          border: Border.all(color: themeColor.withOpacity(0.2)),
          borderRadius: BorderRadius.circular(16),
        ),
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(title, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 12)),
                if (label != null)
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(color: themeColor.withOpacity(0.2), borderRadius: BorderRadius.circular(4)),
                    child: Text(label.toUpperCase(), style: TextStyle(color: themeColor, fontSize: 8, fontWeight: FontWeight.bold)),
                  )
                else
                  Text('[Default]', style: TextStyle(color: Colors.grey.withOpacity(0.5), fontSize: 10, fontWeight: FontWeight.bold)),
              ],
            ),
            const SizedBox(height: 12),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(color: Colors.black45, borderRadius: BorderRadius.circular(4)),
              child: Text(address, style: TextStyle(color: themeColor, fontFamily: 'monospace', fontSize: 10), overflow: TextOverflow.ellipsis),
            ),
          ],
        ),
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
