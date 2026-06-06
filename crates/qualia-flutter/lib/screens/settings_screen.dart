import 'package:flutter/material.dart';
// import '../src/rust/api.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  // Stub models for now since bridge is not compiled yet
  String _storagePath = 'C:\\QualiaData';
  double _storageQuotaGb = 50.0;
  String _status = '';
  
  final List<Map<String, dynamic>> _taxRecipients = [
    {'label': 'Cooperative Infrastructure Fund', 'ilp': '\$ilp.qualia.coop/infrastructure', 'share': 40.0, 'nym': false},
    {'label': 'Digital Rights Legal Defence', 'ilp': '\$ilp.qualia.coop/legal-defence', 'share': 30.0, 'nym': false},
    {'label': 'Open Source Sustainability Pool', 'ilp': '\$ilp.qualia.coop/oss-sustainability', 'share': 20.0, 'nym': false},
    {'label': 'Disaster Recovery Reserve', 'ilp': '\$ilp.qualia.coop/disaster-reserve', 'share': 10.0, 'nym': true},
  ];

  @override
  void initState() {
    super.initState();
    // In future: load config from get_config() and get_tax_suite()
  }

  void _handleSave() {
    setState(() {
      _status = 'Configuration saved.';
    });
    Future.delayed(const Duration(seconds: 3), () {
      if (mounted) setState(() => _status = '');
    });
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _buildPanel(
            context,
            title: 'System Configuration',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Data Storage Path', style: TextStyle(color: Colors.grey)),
                const SizedBox(height: 8),
                TextField(
                  controller: TextEditingController(text: _storagePath)..selection = TextSelection.collapsed(offset: _storagePath.length),
                  onChanged: (v) => _storagePath = v,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    filled: true,
                  ),
                ),
                const SizedBox(height: 4),
                const Text('Models, ontologies, and vector databases will be stored here.', style: TextStyle(fontSize: 12, color: Colors.grey)),
                
                const SizedBox(height: 24),
                const Text('Storage Quota (GB)', style: TextStyle(color: Colors.grey)),
                Slider(
                  value: _storageQuotaGb,
                  min: 1,
                  max: 500,
                  activeColor: const Color(0xFFFFD700),
                  onChanged: (v) => setState(() => _storageQuotaGb = v),
                ),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text('1 GB', style: TextStyle(color: Color(0xFFFFD700), fontSize: 12)),
                    Text('${_storageQuotaGb.toInt()} GB Selected', style: const TextStyle(color: Color(0xFFFFD700), fontSize: 12)),
                    const Text('500 GB', style: TextStyle(color: Color(0xFFFFD700), fontSize: 12)),
                  ],
                ),
                const SizedBox(height: 24),
                if (_status.isNotEmpty) ...[
                  Text(_status, style: const TextStyle(color: Colors.greenAccent)),
                  const SizedBox(height: 16),
                ],
                ElevatedButton(
                  onPressed: _handleSave,
                  style: ElevatedButton.styleFrom(
                    backgroundColor: Colors.greenAccent.withOpacity(0.1),
                    foregroundColor: Colors.greenAccent,
                  ),
                  child: const Text('Save Configuration'),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          _buildPanel(
            context,
            title: '12% TAX ROUTER — RECIPIENT SUITE',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Every accepted ILP payment is automatically split: 12% is dispatched as micropayments to the addresses below.', style: TextStyle(color: Colors.grey, fontSize: 14)),
                const SizedBox(height: 16),
                Container(
                  decoration: BoxDecoration(
                    color: Colors.black38,
                    border: Border.all(color: Colors.white12),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    children: _taxRecipients.map((r) {
                      return Padding(
                        padding: const EdgeInsets.only(bottom: 8.0),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Row(
                              children: [
                                Text(r['label']),
                                if (r['nym']) ...[
                                  const SizedBox(width: 8),
                                  Container(
                                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                                    decoration: BoxDecoration(
                                      color: Colors.cyanAccent.withOpacity(0.1),
                                      border: Border.all(color: Colors.cyanAccent.withOpacity(0.2)),
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    child: const Text('NYM', style: TextStyle(color: Colors.cyanAccent, fontSize: 10)),
                                  ),
                                ]
                              ],
                            ),
                            Text('${r['share']}%', style: const TextStyle(color: Color(0xFFFFD700))),
                          ],
                        ),
                      );
                    }).toList(),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildPanel(BuildContext context, {required String title, required Widget child}) {
    return Container(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.white10),
      ),
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold)),
          const Divider(height: 32, color: Colors.white10),
          child,
        ],
      ),
    );
  }
}
