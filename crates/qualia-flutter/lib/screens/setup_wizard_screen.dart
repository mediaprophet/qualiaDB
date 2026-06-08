import 'package:flutter/material.dart';

import '../src/rust/api/qualia_api.dart' as api;

/// First-run setup wizard — mirrors Tauri `SetupWizard` in App.tsx.
class SetupWizardOverlay extends StatefulWidget {
  final VoidCallback onComplete;

  const SetupWizardOverlay({super.key, required this.onComplete});

  @override
  State<SetupWizardOverlay> createState() => _SetupWizardOverlayState();
}

class _SetupWizardOverlayState extends State<SetupWizardOverlay> {
  final _pathController = TextEditingController();
  double _quotaGb = 50;
  bool _saving = false;
  String _error = '';

  @override
  void initState() {
    super.initState();
    _loadDefaults();
  }

  Future<void> _loadDefaults() async {
    try {
      final config = await api.getConfig();
      if (mounted) {
        _pathController.text = config.storagePath;
        setState(() => _quotaGb = config.storageQuotaGb.toInt().toDouble());
      }
    } catch (_) {}
  }

  @override
  void dispose() {
    _pathController.dispose();
    super.dispose();
  }

  Future<void> _initialize() async {
    final path = _pathController.text.trim();
    if (path.isEmpty) return;

    setState(() {
      _saving = true;
      _error = '';
    });

    try {
      final existing = await api.getConfig();
      await api.saveConfig(
        newConfig: api.AgentConfig(
          storagePath: path,
          storageQuotaGb: BigInt.from(_quotaGb.toInt()),
          baseConnectivityCostIlp: existing.baseConnectivityCostIlp,
        ),
      );
      widget.onComplete();
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = '$e';
          _saving = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.black.withValues(alpha: 0.92),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: Card(
            margin: const EdgeInsets.all(24),
            child: Padding(
              padding: const EdgeInsets.all(32),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    'QualiaDB',
                    style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                      color: Theme.of(context).colorScheme.primary,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'First-time setup — configure where your agent stores its data.',
                    style: TextStyle(color: Colors.grey),
                  ),
                  const SizedBox(height: 24),
                  const Text('Data Storage Path', style: TextStyle(color: Colors.grey, fontSize: 12)),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _pathController,
                    decoration: const InputDecoration(border: OutlineInputBorder()),
                  ),
                  const SizedBox(height: 4),
                  const Text(
                    'Models, ontologies, and vector databases will be stored here.',
                    style: TextStyle(fontSize: 11, color: Colors.grey),
                  ),
                  const SizedBox(height: 20),
                  const Text('Storage Quota (GB)', style: TextStyle(color: Colors.grey, fontSize: 12)),
                  Slider(
                    value: _quotaGb,
                    min: 1,
                    max: 500,
                    divisions: 499,
                    label: '${_quotaGb.toInt()} GB',
                    onChanged: _saving ? null : (v) => setState(() => _quotaGb = v),
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text('1 GB', style: TextStyle(fontSize: 11, color: Colors.grey)),
                      Text('${_quotaGb.toInt()} GB', style: const TextStyle(color: Color(0xFFFFD700))),
                      const Text('500 GB', style: TextStyle(fontSize: 11, color: Colors.grey)),
                    ],
                  ),
                  if (_error.isNotEmpty) ...[
                    const SizedBox(height: 12),
                    Text(_error, style: const TextStyle(color: Colors.redAccent, fontSize: 12)),
                  ],
                  const SizedBox(height: 24),
                  ElevatedButton(
                    onPressed: _saving || _pathController.text.trim().isEmpty ? null : _initialize,
                    child: Text(_saving ? 'Initializing…' : 'Initialize Qualia'),
                  ),
                  const SizedBox(height: 12),
                  const Text(
                    'After setup, open Principal Identifiers & Verifiable Claims to generate your principal identifier root.',
                    textAlign: TextAlign.center,
                    style: TextStyle(fontSize: 11, color: Colors.grey),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
