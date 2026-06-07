import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/qpu_feature_service.dart';
import '../src/rust/api/qualia_api.dart' as api;

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  final _pathController = TextEditingController();
  double _storageQuotaGb = 50.0;
  BigInt _baseConnectivityCostIlp = BigInt.from(5000);
  String _status = '';
  String _error = '';
  bool _loading = true;
  api.TaxRecipientSuite? _taxSuite;
  List<_EditableRecipient> _editableRecipients = [];

  // QPU Oracle (visible only when unlocked via chat command)
  api.QpuOracleSettings? _qpuSettings;
  final _ibmTokenController = TextEditingController();
  final _dwaveTokenController = TextEditingController();
  bool _qpuSaving = false;

  int get _shareTotal =>
      _editableRecipients.fold(0, (sum, r) => sum + (int.tryParse(r.shareController.text) ?? 0));

  bool get _shareValid => _shareTotal == 100;

  @override
  void initState() {
    super.initState();
    _loadConfig();
  }

  Future<void> _loadQpuIfNeeded() async {
    if (!ref.read(qpuFeatureUnlockedProvider) || _qpuSettings != null) return;
    try {
      final qpu = await api.getQpuSettings();
      if (mounted) setState(() => _qpuSettings = qpu);
    } catch (_) {}
  }

  @override
  void dispose() {
    _pathController.dispose();
    _ibmTokenController.dispose();
    _dwaveTokenController.dispose();
    for (final r in _editableRecipients) {
      r.dispose();
    }
    super.dispose();
  }

  void _syncEditableFromSuite(api.TaxRecipientSuite suite) {
    for (final r in _editableRecipients) {
      r.dispose();
    }
    _editableRecipients = suite.recipients
        .map((r) => _EditableRecipient.fromRecipient(r))
        .toList();
  }

  api.TaxRecipientSuite _buildSuiteFromEditable() {
    final jurisdiction = _taxSuite?.jurisdictionDid ?? 'did:q42:cooperative-default';
    return api.TaxRecipientSuite(
      jurisdictionDid: jurisdiction,
      recipients: _editableRecipients
          .map(
            (r) => api.TaxRecipient(
              label: r.labelController.text.trim(),
              ilpAddress: r.addressController.text.trim(),
              sharePercent: BigInt.from(int.tryParse(r.shareController.text) ?? 0),
              useNym: r.useNym,
            ),
          )
          .toList(),
    );
  }

  void _addRecipient() {
    setState(() {
      _editableRecipients.add(_EditableRecipient.empty());
    });
  }

  void _removeRecipient(int index) {
    setState(() {
      _editableRecipients[index].dispose();
      _editableRecipients.removeAt(index);
    });
  }

  Future<void> _loadConfig() async {
    try {
      final config = await api.getConfig();
      final suite = await api.getTaxSuite();
      api.QpuOracleSettings? qpu;
      if (await api.isQpuFeatureUnlocked()) {
        qpu = await api.getQpuSettings();
      }
      if (mounted) {
        setState(() {
          _pathController.text = config.storagePath;
          _storageQuotaGb = config.storageQuotaGb.toInt().toDouble();
          _baseConnectivityCostIlp = config.baseConnectivityCostIlp;
          _taxSuite = suite;
          _syncEditableFromSuite(suite);
          _qpuSettings = qpu;
          _loading = false;
        });
        if (qpu != null) {
          ref.read(qpuFeatureUnlockedProvider.notifier).setUnlocked(qpu.featureUnlocked);
        }
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = '$e';
          _loading = false;
        });
      }
    }
  }

  Future<void> _saveQpuSettings() async {
    final qpu = _qpuSettings;
    if (qpu == null) return;
    setState(() {
      _qpuSaving = true;
      _error = '';
      _status = '';
    });
    try {
      final ibmText = _ibmTokenController.text.trim();
      final dwaveText = _dwaveTokenController.text.trim();
      final updated = await api.saveQpuSettings(
        input: api.QpuOracleSettingsInput(
          maxShotsPerTask: qpu.maxShotsPerTask,
          fallbackToClassical: qpu.fallbackToClassical,
          enableQuboRouting: qpu.enableQuboRouting,
          enableDftGroundState: qpu.enableDftGroundState,
          enableDefeasibleResolution: qpu.enableDefeasibleResolution,
          ibmToken: ibmText.isEmpty ? null : ibmText,
          dwaveToken: dwaveText.isEmpty ? null : dwaveText,
        ),
      );
      if (mounted) {
        setState(() {
          _qpuSettings = updated;
          _ibmTokenController.clear();
          _dwaveTokenController.clear();
          _status = 'QPU Oracle configuration saved.';
          _qpuSaving = false;
        });
        Future.delayed(const Duration(seconds: 3), () {
          if (mounted) setState(() => _status = '');
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = '$e';
          _qpuSaving = false;
        });
      }
    }
  }

  Future<void> _disableQpuFeature() async {
    final result = await api.handleQpuChatCommand(text: '[disable_QPU]');
    ref.read(qpuFeatureUnlockedProvider.notifier).setUnlocked(result.featureUnlocked);
    if (mounted) {
      setState(() {
        _qpuSettings = null;
        _ibmTokenController.clear();
        _dwaveTokenController.clear();
      });
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('QPU Oracle hidden. Type [enable_QPU] in Chat to restore.')),
      );
    }
  }

  Future<void> _handleSave() async {
    setState(() {
      _status = '';
      _error = '';
    });
    try {
      if (!_shareValid) {
        setState(() => _error = 'Recipient shares must sum to 100% (currently $_shareTotal%).');
        return;
      }
      await api.saveConfig(
        newConfig: api.AgentConfig(
          storagePath: _pathController.text.trim(),
          storageQuotaGb: BigInt.from(_storageQuotaGb.toInt()),
          baseConnectivityCostIlp: _baseConnectivityCostIlp,
        ),
      );
      final suite = _buildSuiteFromEditable();
      await api.saveTaxSuite(suite: suite);
      if (mounted) {
        setState(() {
          _taxSuite = suite;
          _status = 'Configuration saved.';
        });
        Future.delayed(const Duration(seconds: 3), () {
          if (mounted) setState(() => _status = '');
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }

    final recipients = _editableRecipients;
    final qpuUnlocked = ref.watch(qpuFeatureUnlockedProvider);
    final qpu = _qpuSettings;

    ref.listen<bool>(qpuFeatureUnlockedProvider, (prev, next) {
      if (next && _qpuSettings == null) _loadQpuIfNeeded();
      if (!next && _qpuSettings != null) setState(() => _qpuSettings = null);
    });
    if (qpuUnlocked && qpu == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _loadQpuIfNeeded());
    }

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
                  controller: _pathController,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    filled: true,
                  ),
                ),
                const SizedBox(height: 4),
                const Text(
                  'Models, ontologies, and vector databases will be stored here.',
                  style: TextStyle(fontSize: 12, color: Colors.grey),
                ),
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
                    Text(
                      '${_storageQuotaGb.toInt()} GB Selected',
                      style: const TextStyle(color: Color(0xFFFFD700), fontSize: 12),
                    ),
                    const Text('500 GB', style: TextStyle(color: Color(0xFFFFD700), fontSize: 12)),
                  ],
                ),
                const SizedBox(height: 24),
                if (_status.isNotEmpty)
                  Text(_status, style: const TextStyle(color: Colors.greenAccent)),
                if (_error.isNotEmpty)
                  Text(_error, style: const TextStyle(color: Colors.redAccent)),
                const SizedBox(height: 16),
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
                const Text(
                  'Every accepted ILP payment is automatically split: 12% is dispatched as micropayments to the addresses below.',
                  style: TextStyle(color: Colors.grey, fontSize: 14),
                ),
                const SizedBox(height: 16),
                Container(
                  decoration: BoxDecoration(
                    color: Colors.black38,
                    border: Border.all(color: Colors.white12),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    children: [
                      for (var i = 0; i < recipients.length; i++) ...[
                        if (i > 0) const Divider(height: 24, color: Colors.white12),
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(
                              flex: 2,
                              child: TextField(
                                controller: recipients[i].labelController,
                                decoration: const InputDecoration(
                                  labelText: 'Label',
                                  isDense: true,
                                  border: OutlineInputBorder(),
                                ),
                                onChanged: (_) => setState(() {}),
                              ),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              flex: 3,
                              child: TextField(
                                controller: recipients[i].addressController,
                                decoration: const InputDecoration(
                                  labelText: 'ILP address',
                                  isDense: true,
                                  border: OutlineInputBorder(),
                                ),
                                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                                onChanged: (_) => setState(() {}),
                              ),
                            ),
                            const SizedBox(width: 12),
                            SizedBox(
                              width: 72,
                              child: TextField(
                                controller: recipients[i].shareController,
                                keyboardType: TextInputType.number,
                                decoration: const InputDecoration(
                                  labelText: '%',
                                  isDense: true,
                                  border: OutlineInputBorder(),
                                ),
                                onChanged: (_) => setState(() {}),
                              ),
                            ),
                            const SizedBox(width: 8),
                            Column(
                              children: [
                                const Text('NYM', style: TextStyle(fontSize: 10, color: Colors.grey)),
                                Switch(
                                  value: recipients[i].useNym,
                                  onChanged: (v) => setState(() => recipients[i].useNym = v),
                                ),
                              ],
                            ),
                            IconButton(
                              icon: const Icon(Icons.delete_outline, color: Colors.redAccent),
                              tooltip: 'Remove recipient',
                              onPressed: recipients.length > 1 ? () => _removeRecipient(i) : null,
                            ),
                          ],
                        ),
                      ],
                    ],
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    TextButton.icon(
                      onPressed: _addRecipient,
                      icon: const Icon(Icons.add),
                      label: const Text('Add recipient'),
                    ),
                    const Spacer(),
                    Text(
                      'Total: $_shareTotal%',
                      style: TextStyle(
                        color: _shareValid ? const Color(0xFFFFD700) : Colors.redAccent,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: _shareValid ? _handleSave : null,
                  style: ElevatedButton.styleFrom(
                    backgroundColor: Colors.greenAccent.withOpacity(0.1),
                    foregroundColor: Colors.greenAccent,
                  ),
                  child: const Text('Save Tax Suite'),
                ),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: () async {
                    try {
                      final result = await api.dispatchTaxPayment(grossAmountMicroCents: BigInt.from(1000000));
                      if (mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text('Tax dispatch: sent ${result.totalSent}, queued ${result.totalQueued}')),
                        );
                      }
                    } catch (e) {
                      if (mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Dispatch failed: $e')));
                      }
                    }
                  },
                  child: const Text('Test ILP Tax Dispatch'),
                ),
              ],
            ),
          ),
          if (qpuUnlocked && qpu != null) ...[
            const SizedBox(height: 24),
            _buildQpuOraclePanel(context, qpu),
          ],
        ],
      ),
    );
  }

  Widget _buildQpuOraclePanel(BuildContext context, api.QpuOracleSettings qpu) {
    return _buildPanel(
      context,
      title: 'QPU ORACLE — SOVEREIGN REMOTE COMPUTE',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Bring-your-own-key access to IBM Quantum (gate-model) and D-Wave Leap (annealer). '
            'Only anonymized numeric matrices egress; classified assertions are blocked by the Sentinel.',
            style: TextStyle(color: Colors.grey, fontSize: 14),
          ),
          const SizedBox(height: 16),
          Row(
            children: [
              _quotaChip('IBM', qpu.ibmQuotaMinutesRemaining, 10),
              const SizedBox(width: 12),
              _quotaChip('D-Wave', qpu.dwaveQuotaMinutesRemaining, 1),
            ],
          ),
          const SizedBox(height: 24),
          TextField(
            controller: _ibmTokenController,
            obscureText: true,
            decoration: InputDecoration(
              labelText: 'IBM Quantum API Token',
              hintText: qpu.ibmTokenConfigured ? '••••••••  (configured — enter to replace)' : 'Paste IBM_QUANTUM_TOKEN',
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _dwaveTokenController,
            obscureText: true,
            decoration: InputDecoration(
              labelText: 'D-Wave Leap API Token',
              hintText: qpu.dwaveTokenConfigured ? '••••••••  (configured — enter to replace)' : 'Paste DWAVE_API_TOKEN',
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 24),
          const Text('Max shots per task (SHACL limit: 1000)', style: TextStyle(color: Colors.grey)),
          Slider(
            value: qpu.maxShotsPerTask.toDouble(),
            min: 1,
            max: 1000,
            divisions: 99,
            activeColor: const Color(0xFF5D5FEF),
            label: '${qpu.maxShotsPerTask}',
            onChanged: (v) => setState(() {
              _qpuSettings = api.QpuOracleSettings(
                featureUnlocked: qpu.featureUnlocked,
                ibmTokenConfigured: qpu.ibmTokenConfigured,
                dwaveTokenConfigured: qpu.dwaveTokenConfigured,
                maxShotsPerTask: v.round(),
                fallbackToClassical: qpu.fallbackToClassical,
                enableQuboRouting: qpu.enableQuboRouting,
                enableDftGroundState: qpu.enableDftGroundState,
                enableDefeasibleResolution: qpu.enableDefeasibleResolution,
                ibmQuotaMinutesRemaining: qpu.ibmQuotaMinutesRemaining,
                dwaveQuotaMinutesRemaining: qpu.dwaveQuotaMinutesRemaining,
              );
            }),
          ),
          SwitchListTile(
            title: const Text('Fallback to classical approximation when quota exhausted'),
            value: qpu.fallbackToClassical,
            onChanged: (v) => setState(() {
              _qpuSettings = _copyQpu(qpu, fallbackToClassical: v);
            }),
          ),
          const Divider(color: Colors.white12),
          const Text('Prioritized invocations', style: TextStyle(fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          SwitchListTile(
            title: const Text('QUBO constraint routing (D-Wave)'),
            subtitle: const Text('Safe-house / crisis routing optimization'),
            value: qpu.enableQuboRouting,
            onChanged: (v) => setState(() {
              _qpuSettings = _copyQpu(qpu, enableQuboRouting: v);
            }),
          ),
          SwitchListTile(
            title: const Text('DFT ground-state energies (IBM)'),
            subtitle: const Text('Variational Quantum Eigensolver for molecular states'),
            value: qpu.enableDftGroundState,
            onChanged: (v) => setState(() {
              _qpuSettings = _copyQpu(qpu, enableDftGroundState: v);
            }),
          ),
          SwitchListTile(
            title: const Text('Defeasible logic conflict resolution (IBM)'),
            subtitle: const Text('Probabilistic inference for competing obligations'),
            value: qpu.enableDefeasibleResolution,
            onChanged: (v) => setState(() {
              _qpuSettings = _copyQpu(qpu, enableDefeasibleResolution: v);
            }),
          ),
          const SizedBox(height: 16),
          Row(
            children: [
              ElevatedButton(
                onPressed: _qpuSaving ? null : _saveQpuSettings,
                style: ElevatedButton.styleFrom(
                  backgroundColor: const Color(0xFF5D5FEF).withValues(alpha: 0.2),
                  foregroundColor: const Color(0xFF5D5FEF),
                ),
                child: _qpuSaving
                    ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Text('Save QPU Configuration'),
              ),
              const SizedBox(width: 12),
              TextButton(
                onPressed: _disableQpuFeature,
                child: const Text('Hide QPU Oracle', style: TextStyle(color: Colors.redAccent)),
              ),
            ],
          ),
        ],
      ),
    );
  }

  api.QpuOracleSettings _copyQpu(
    api.QpuOracleSettings qpu, {
    bool? fallbackToClassical,
    bool? enableQuboRouting,
    bool? enableDftGroundState,
    bool? enableDefeasibleResolution,
  }) {
    return api.QpuOracleSettings(
      featureUnlocked: qpu.featureUnlocked,
      ibmTokenConfigured: qpu.ibmTokenConfigured,
      dwaveTokenConfigured: qpu.dwaveTokenConfigured,
      maxShotsPerTask: qpu.maxShotsPerTask,
      fallbackToClassical: fallbackToClassical ?? qpu.fallbackToClassical,
      enableQuboRouting: enableQuboRouting ?? qpu.enableQuboRouting,
      enableDftGroundState: enableDftGroundState ?? qpu.enableDftGroundState,
      enableDefeasibleResolution: enableDefeasibleResolution ?? qpu.enableDefeasibleResolution,
      ibmQuotaMinutesRemaining: qpu.ibmQuotaMinutesRemaining,
      dwaveQuotaMinutesRemaining: qpu.dwaveQuotaMinutesRemaining,
    );
  }

  Widget _quotaChip(String label, double remaining, double total) {
    return Chip(
      avatar: const Icon(Icons.timer_outlined, size: 16),
      label: Text('$label: ${remaining.toStringAsFixed(1)} / $total min/mo'),
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

class _EditableRecipient {
  final TextEditingController labelController;
  final TextEditingController addressController;
  final TextEditingController shareController;
  bool useNym;

  _EditableRecipient({
    required this.labelController,
    required this.addressController,
    required this.shareController,
    required this.useNym,
  });

  factory _EditableRecipient.fromRecipient(api.TaxRecipient r) {
    return _EditableRecipient(
      labelController: TextEditingController(text: r.label),
      addressController: TextEditingController(text: r.ilpAddress),
      shareController: TextEditingController(text: r.sharePercent.toString()),
      useNym: r.useNym,
    );
  }

  factory _EditableRecipient.empty() {
    return _EditableRecipient(
      labelController: TextEditingController(),
      addressController: TextEditingController(text: r'$ilp.qualia.coop/'),
      shareController: TextEditingController(text: '0'),
      useNym: false,
    );
  }

  void dispose() {
    labelController.dispose();
    addressController.dispose();
    shareController.dispose();
  }
}
