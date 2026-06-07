import 'dart:convert';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:qualia_flutter/src/rust/api/qualia_api.dart' as api;

import '../main.dart' show shellNavIndexProvider;

class WalletScreen extends ConsumerStatefulWidget {
  const WalletScreen({super.key});

  @override
  ConsumerState<WalletScreen> createState() => _WalletScreenState();
}

class _WalletScreenState extends ConsumerState<WalletScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabs;
  List<api.CoinBalance> _balances = [];
  List<api.TxRecord> _transactions = [];
  List<api.TokenEntry> _tokens = [];
  bool _loading = true;
  bool _nymActive = false;
  bool _starkActive = false;
  bool _privacyExpanded = false;
  bool _hasIdentity = false;
  List<dynamic> _portfolio = [];

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 3, vsync: this);
    _loadAll();
  }

  @override
  void dispose() {
    _tabs.dispose();
    super.dispose();
  }

  Future<void> _loadAll() async {
    setState(() => _loading = true);
    try {
      final balances = await api.getCoinBalances();
      final txs = await api.getTransactionHistory(ticker: 'ALL');
      final tokens = await api.getTokens();
      final status = await api.getWalletStatus();
      final portfolioJson = await api.fetchWalletPortfolio();
      final portfolio = jsonDecode(portfolioJson) as List<dynamic>;
      final identityJson = await api.loadIdentity();
      if (mounted) {
        setState(() {
          _balances = balances;
          _transactions = txs;
          _tokens = tokens;
          _portfolio = portfolio;
          _nymActive = status.nymConnected;
          _hasIdentity = identityJson != null && identityJson.isNotEmpty;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _copyAddress(String address) {
    if (address.isEmpty) return;
    Clipboard.setData(ClipboardData(text: address));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Address copied')),
    );
  }

  String _shortAddress(String address) {
    if (address.isEmpty) return 'No address — create identity first';
    if (address.length <= 24) return address;
    return '${address.substring(0, 12)}…${address.substring(address.length - 8)}';
  }

  void _openCredentials() {
    ref.read(shellNavIndexProvider.notifier).state = 6;
  }

  void _showSendDialog(api.CoinBalance coin) {
    if (coin.address.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Generate your identity in Credentials before sending.'),
        ),
      );
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('On-chain send is not connected yet — receive/copy addresses only.'),
      ),
    );
  }

  void _showReceiveDialog(api.CoinBalance coin) {
    if (coin.address.isEmpty) {
      _openCredentials();
      return;
    }
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Receive ${coin.ticker}'),
        content: SelectableText(coin.address),
        actions: [
          TextButton(
            onPressed: () {
              Clipboard.setData(ClipboardData(text: coin.address));
              Navigator.pop(ctx);
              ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Address copied')));
            },
            child: const Text('Copy'),
          ),
        ],
      ),
    );
  }

  void _showAddTokenDialog() {
    final symbolCtrl = TextEditingController();
    final nameCtrl = TextEditingController();
    final contractCtrl = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Add ERC-20 / Custom Token'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(controller: symbolCtrl, decoration: const InputDecoration(labelText: 'Symbol')),
            TextField(controller: nameCtrl, decoration: const InputDecoration(labelText: 'Name')),
            TextField(controller: contractCtrl, decoration: const InputDecoration(labelText: 'Contract address')),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () async {
              Navigator.pop(ctx);
              try {
                await api.addToken(
                  chain: 'Ethereum',
                  tokenType: 'ERC-20',
                  contract: contractCtrl.text.trim(),
                  symbol: symbolCtrl.text.trim(),
                  name: nameCtrl.text.trim(),
                  decimals: 18,
                );
                await _loadAll();
              } catch (e) {
                if (mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
                }
              }
            },
            child: const Text('Add'),
          ),
        ],
      ),
    );
  }

  Future<void> _removeToken(api.TokenEntry token) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Remove ${token.symbol}?'),
        content: Text('Remove ${token.name} from your watch list?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: ElevatedButton.styleFrom(backgroundColor: Colors.redAccent),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    try {
      await api.removeToken(id: token.id);
      await _loadAll();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Removed ${token.symbol}')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Center(child: CircularProgressIndicator(color: Color(0xFF00F0FF)));
    }

    final totalUsd = _balances.fold(0.0, (sum, item) => sum + item.fiatUsd);

    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.account_balance_wallet, color: Color(0xFF00F0FF), size: 28),
              const SizedBox(width: 12),
              const Text('Decentralized Wallet', style: TextStyle(color: Colors.white, fontSize: 24, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
              const Spacer(),
              IconButton(icon: const Icon(Icons.refresh), onPressed: _loadAll),
              IconButton(
                icon: const Icon(Icons.add_circle_outline),
                tooltip: 'Add token',
                onPressed: _showAddTokenDialog,
              ),
            ],
          ),
          const SizedBox(height: 16),
          if (!_hasIdentity)
            MaterialBanner(
              content: const Text(
                'No wallet identity yet. Open Credentials (key icon) to generate '
                'a seed phrase and derive BTC, XMR, and other receive addresses.',
              ),
              leading: const Icon(Icons.key_outlined),
              actions: [
                TextButton(onPressed: _openCredentials, child: const Text('Credentials')),
              ],
            )
          else
            MaterialBanner(
              content: const Text(
                'Receive addresses are derived from your identity. '
                'On-chain balance sync and send are not connected yet.',
              ),
              leading: const Icon(Icons.info_outline),
              actions: [
                TextButton(onPressed: _openCredentials, child: const Text('View keys')),
              ],
            ),
          const SizedBox(height: 12),
          _buildOptionalPrivacySection(),
          const SizedBox(height: 12),
          _buildGlassContainer(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('Total Portfolio Value', style: TextStyle(color: Colors.grey, fontSize: 14)),
                  Text('\$${totalUsd.toStringAsFixed(2)}', style: const TextStyle(color: Colors.white, fontSize: 40, fontWeight: FontWeight.bold)),
                  const SizedBox(height: 4),
                  Text(
                    _hasIdentity ? 'Balances show 0 until chain sync ships' : 'Create identity to show addresses',
                    style: const TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
          TabBar(
            controller: _tabs,
            tabs: const [Tab(text: 'Assets'), Tab(text: 'History'), Tab(text: 'Tokens')],
          ),
          const SizedBox(height: 12),
          Expanded(
            child: TabBarView(
              controller: _tabs,
              children: [
                _buildAssetsTab(),
                _buildHistoryTab(),
                _buildTokensTab(),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildOptionalPrivacySection() {
    return _buildGlassContainer(
      child: Theme(
        data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
        child: ExpansionTile(
          initiallyExpanded: _privacyExpanded,
          onExpansionChanged: (v) => setState(() => _privacyExpanded = v),
          tilePadding: const EdgeInsets.symmetric(horizontal: 16),
          title: const Text(
            'Optional privacy routing',
            style: TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.w600),
          ),
          subtitle: const Text(
            'Nym mixnet and STARK proving are off by default. Enable only if you need them.',
            style: TextStyle(color: Colors.grey, fontSize: 11),
          ),
          children: [
            SwitchListTile(
              title: const Text('Nym mixnet relay', style: TextStyle(color: Colors.white, fontSize: 13)),
              subtitle: const Text(
                'Routes selected payments through the mixnet. Adds a NYM asset when enabled.',
                style: TextStyle(color: Colors.grey, fontSize: 11),
              ),
              value: _nymActive,
              onChanged: (_) async {
                final v = await api.toggleNymRelay();
                if (!mounted) return;
                setState(() => _nymActive = v);
                await _loadAll();
              },
            ),
            SwitchListTile(
              title: const Text('STARK prover', style: TextStyle(color: Colors.white, fontSize: 13)),
              subtitle: const Text(
                'Zero-knowledge proof generation for classified graph fragments.',
                style: TextStyle(color: Colors.grey, fontSize: 11),
              ),
              value: _starkActive,
              onChanged: (_) async {
                final v = await api.toggleStarkProver();
                if (mounted) setState(() => _starkActive = v);
              },
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAssetsTab() {
    return ListView.builder(
      itemCount: _balances.length,
      itemBuilder: (context, index) {
        final item = _balances[index];
        final hasAddress = item.address.isNotEmpty;
        return Container(
          margin: const EdgeInsets.only(bottom: 12),
          child: _buildGlassContainer(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          item.coin,
                          style: const TextStyle(
                            color: Colors.white,
                            fontWeight: FontWeight.bold,
                            fontSize: 16,
                          ),
                        ),
                      ),
                      Text(
                        '${item.balanceDisplay} ${item.ticker}',
                        style: const TextStyle(
                          color: Colors.grey,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      Expanded(
                        child: SelectableText(
                          _shortAddress(item.address),
                          style: TextStyle(
                            color: hasAddress ? const Color(0xFF00F0FF) : Colors.orange,
                            fontFamily: 'monospace',
                            fontSize: 12,
                          ),
                        ),
                      ),
                      if (hasAddress)
                        IconButton(
                          icon: const Icon(Icons.copy, size: 18),
                          tooltip: 'Copy address',
                          onPressed: () => _copyAddress(item.address),
                        ),
                    ],
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.arrow_upward, size: 18),
                        tooltip: 'Send',
                        onPressed: () => _showSendDialog(item),
                      ),
                      IconButton(
                        icon: const Icon(Icons.arrow_downward, size: 18),
                        tooltip: 'Receive',
                        onPressed: () => _showReceiveDialog(item),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildHistoryTab() {
    return ListView.builder(
      itemCount: _transactions.length,
      itemBuilder: (context, i) {
        final tx = _transactions[i];
        final isIn = tx.direction == 'in';
        return ListTile(
          leading: Icon(isIn ? Icons.call_received : Icons.call_made, color: isIn ? Colors.green : Colors.orange),
          title: Text('${tx.amount} ${tx.ticker}'),
          subtitle: Text('${tx.label} • ${tx.timestamp}'),
          trailing: Text(tx.status, style: const TextStyle(fontSize: 11)),
        );
      },
    );
  }

  Widget _buildTokensTab() {
    if (_tokens.isEmpty && _portfolio.isEmpty) {
      return const Center(child: Text('No custom tokens — use + to add one.', style: TextStyle(color: Colors.grey)));
    }

    return ListView.builder(
      itemCount: _tokens.length,
      itemBuilder: (context, i) {
        final t = _tokens[i];
        Map<String, dynamic>? portfolioMatch;
        for (final p in _portfolio) {
          final map = p as Map<String, dynamic>;
          if (map['symbol'] == t.symbol || map['contract'] == t.contract) {
            portfolioMatch = map;
            break;
          }
        }
        final balance = portfolioMatch?['balance']?.toString() ?? t.balance;
        return ListTile(
          title: Text('${t.symbol} — ${t.name}'),
          subtitle: Text('${t.chain} · ${t.contract}'),
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(balance),
              IconButton(
                icon: const Icon(Icons.delete_outline, color: Colors.redAccent),
                tooltip: 'Remove token',
                onPressed: () => _removeToken(t),
              ),
            ],
          ),
        );
      },
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
