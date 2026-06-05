import 'dart:ui';
import 'package:flutter/material.dart';

class WalletScreen extends StatelessWidget {
  const WalletScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final balances = [
      {'coin': 'eCash', 'ticker': 'XEC', 'balance': '1,250,000.00', 'usd': '\$245.00', 'color': const Color(0xFF00FF88)},
      {'coin': 'Bitcoin', 'ticker': 'BTC', 'balance': '0.00450000', 'usd': '\$441.00', 'color': const Color(0xFFFF9900)},
      {'coin': 'Ethereum', 'ticker': 'ETH', 'balance': '1.42000000', 'usd': '\$4,260.00', 'color': const Color(0xFFB026FF)},
    ];

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
                  const Text('\$4,946.00', style: TextStyle(color: Colors.white, fontSize: 48, fontWeight: FontWeight.bold, fontFamily: 'monospace')),
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
                final color = item['color'] as Color;
                
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
                          child: Text(item['ticker'] as String, style: TextStyle(color: color, fontWeight: FontWeight.bold, fontSize: 12)),
                        ),
                      ),
                      title: Text(item['coin'] as String, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 18)),
                      subtitle: Text(item['balance'] as String, style: const TextStyle(color: Colors.grey, fontFamily: 'monospace')),
                      trailing: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          Text(item['usd'] as String, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16)),
                          const Text('+2.4%', style: TextStyle(color: Color(0xFF00FF88), fontSize: 12)),
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
