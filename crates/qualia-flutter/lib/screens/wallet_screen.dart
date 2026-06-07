import 'dart:convert';
import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qualia_flutter/src/rust/api/qualia_api.dart' as api;

class WalletScreen extends StatefulWidget {
  const WalletScreen({super.key});

  @override
  State<WalletScreen> createState() => _WalletScreenState();
}

class _WalletScreenState extends State<WalletScreen> with SingleTickerProviderStateMixin {
  late TabController _tabs;
  List<api.CoinBalance> _balances = [];
  List<api.TxRecord> _transactions = [];
  List<api.TokenEntry> _tokens = [];
  bool _loading = true;
  bool _nymActive = false;
  bool _starkActive = false;
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
      if (mounted) {
        setState(() {
          _balances = balances;
          _transactions = txs;
          _tokens = tokens;
          _portfolio = portfolio;
          _nymActive = status.nymConnected;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _showSendDialog(api.CoinBalance coin) {
    final amountCtrl = TextEditingController();
    final addrCtrl = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Send ${coin.ticker}'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(controller: addrCtrl, decoration: const InputDecoration(labelText: 'Recipient address')),
            TextField(controller: amountCtrl, decoration: const InputDecoration(labelText: 'Amount'), keyboardType: TextInputType.number),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(ctx);
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text('Queued send of ${amountCtrl.text} ${coin.ticker} to ${addrCtrl.text}')),
              );
            },
            child: const Text('Send'),
          ),
        ],
      ),
    );
  }

  void _showReceiveDialog(api.CoinBalance coin) {
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
              Switch(
                value: _nymActive,
                onChanged: (_) async {
                  final v = await api.toggleNymRelay();
                  setState(() => _nymActive = v);
                },
              ),
              const Text('NYM', style: TextStyle(fontSize: 11, color: Colors.grey)),
              Switch(
                value: _starkActive,
                onChanged: (_) async {
                  final v = await api.toggleStarkProver();
                  setState(() => _starkActive = v);
                },
              ),
              const Text('STARK', style: TextStyle(fontSize: 11, color: Colors.grey)),
              IconButton(
                icon: const Icon(Icons.add_circle_outline),
                tooltip: 'Add token',
                onPressed: _showAddTokenDialog,
              ),
            ],
          ),
          const SizedBox(height: 16),
          _buildGlassContainer(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('Total Portfolio Value', style: TextStyle(color: Colors.grey, fontSize: 14)),
                  Text('\$${totalUsd.toStringAsFixed(2)}', style: const TextStyle(color: Colors.white, fontSize: 40, fontWeight: FontWeight.bold)),
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

  Widget _buildAssetsTab() {
    return ListView.builder(
      itemCount: _balances.length,
      itemBuilder: (context, index) {
        final item = _balances[index];
        return Container(
          margin: const EdgeInsets.only(bottom: 12),
          child: _buildGlassContainer(
            child: ListTile(
              title: Text(item.coin, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
              subtitle: Text(item.balanceDisplay, style: const TextStyle(color: Colors.grey, fontFamily: 'monospace')),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text('\$${item.fiatUsd.toStringAsFixed(2)}', style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
                  const SizedBox(width: 8),
                  IconButton(icon: const Icon(Icons.arrow_upward, size: 18), onPressed: () => _showSendDialog(item)),
                  IconButton(icon: const Icon(Icons.arrow_downward, size: 18), onPressed: () => _showReceiveDialog(item)),
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
