import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:qualia_flutter/src/rust/api/qualia_api.dart' as api;

class WalletScreen extends StatelessWidget {
  const WalletScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<api.CoinBalance>>(
      future: api.getCoinBalances(),
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.waiting) {
          return const Center(child: CircularProgressIndicator(color: Color(0xFF00F0FF)));
        }
        if (snapshot.hasError) {
          return Center(child: Text('Error: ${snapshot.error}', style: const TextStyle(color: Colors.red)));
        }
        
        final balances = snapshot.data ?? [];
        
        // Calculate total USD for demo
        double totalUsd = balances.fold(0, (sum, item) => sum + item.fiatUsd);

        return Padding(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Row(
                children: [
                  Icon(Icons.account_balance_wallet, color: Color(0xFF00F0FF), size: 28),
                  SizedBox(width: 12),
                  Text('Decentralized Wallet', style: TextStyle(color: Colors.white, fontSize: 24, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                ],
              ),
              const SizedBox(height: 24),
              
              // Total Balance Card
              _buildGlassContainer(
                child: Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(32),
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      colors: [const Color(0xFF00F0FF).withOpacity(0.1), Colors.transparent],
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                    ),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('Total Portfolio Value', style: TextStyle(color: Colors.grey, fontSize: 14, fontFamily: 'monospace')),
                      const SizedBox(height: 8),
                      Text('\$${totalUsd.toStringAsFixed(2)}', style: const TextStyle(color: Colors.white, fontSize: 48, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
                      const SizedBox(height: 24),
                      Row(
                        children: [
                          ElevatedButton.icon(
                            onPressed: () {},
                            icon: const Icon(Icons.arrow_upward, size: 16),
                            label: const Text('Send'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: const Color(0xFF00F0FF).withOpacity(0.2),
                              foregroundColor: const Color(0xFF00F0FF),
                              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                            ),
                          ),
                          const SizedBox(width: 16),
                          ElevatedButton.icon(
                            onPressed: () {},
                            icon: const Icon(Icons.arrow_downward, size: 16),
                            label: const Text('Receive'),
                            style: ElevatedButton.styleFrom(
                              backgroundColor: Colors.white.withOpacity(0.1),
                              foregroundColor: Colors.white,
                              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 32),
              
              const Text('Assets', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
              const SizedBox(height: 16),
              
              Expanded(
                child: ListView.builder(
                  itemCount: balances.length,
                  itemBuilder: (context, index) {
                    final item = balances[index];
                    Color color = const Color(0xFF00FF88);
                    if (item.ticker == 'BTC') color = const Color(0xFFFF9900);
                    if (item.ticker == 'ETH') color = const Color(0xFFB026FF);
                    
                    return Container(
                      margin: const EdgeInsets.only(bottom: 12),
                      child: _buildGlassContainer(
                        child: ListTile(
                          contentPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
                          leading: Container(
                            width: 48,
                            height: 48,
                            decoration: BoxDecoration(
                              color: color.withOpacity(0.1),
                              shape: BoxShape.circle,
                              border: Border.all(color: color.withOpacity(0.3)),
                            ),
                            child: Center(
                              child: Text(item.ticker, style: TextStyle(color: color, fontWeight: FontWeight.bold, fontSize: 12)),
                            ),
                          ),
                          title: Text(item.coin, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 18)),
                          subtitle: Text(item.balanceDisplay, style: const TextStyle(color: Colors.grey, fontFamily: 'monospace')),
                          trailing: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            crossAxisAlignment: CrossAxisAlignment.end,
                            children: [
                              Text('\$${item.fiatUsd.toStringAsFixed(2)}', style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16)),
                              Text('${item.change24h >= 0 ? '+' : ''}${item.change24h}%', style: TextStyle(color: item.change24h >= 0 ? const Color(0xFF00FF88) : Colors.red, fontSize: 12)),
                            ],
                          ),
                        ),
                      ),
                    );
                  },
                ),
              ),
            ],
          ),
        );
      }
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
