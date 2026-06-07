import 'package:flutter/material.dart';
import '../src/rust/api/qualia_api.dart' as api;

class AddressBookScreen extends StatefulWidget {
  const AddressBookScreen({super.key});

  @override
  State<AddressBookScreen> createState() => _AddressBookScreenState();
}

class _AddressBookScreenState extends State<AddressBookScreen> {
  int _tab = 0;
  List<api.FrontDoorBridge> _frontDoors = [];
  List<api.ActorBridge> _actors = [];
  List<api.DelegationRuleBridge> _rules = [];
  bool _loading = true;
  final _doorLabelController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  @override
  void dispose() {
    _doorLabelController.dispose();
    super.dispose();
  }

  Future<void> _loadData() async {
    setState(() => _loading = true);
    try {
      final fd = await api.getFrontDoors();
      final ac = await api.getDirectoryActors();
      final ru = await api.getDelegationRules();
      if (mounted) {
        setState(() {
          _frontDoors = fd;
          _actors = ac;
          _rules = ru;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() => _loading = false);
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Load failed: $e')));
      }
    }
  }

  Future<void> _generateFrontDoor() async {
    final label = _doorLabelController.text.trim();
    if (label.isEmpty) return;
    try {
      await api.generateFrontDoor(label: label);
      _doorLabelController.clear();
      await _loadData();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
      }
    }
  }

  Future<void> _syncFoaf() async {
    final actor = api.ActorBridge(
      id: 'actor-${DateTime.now().millisecondsSinceEpoch}',
      actorType: 'PRACTITIONER',
      name: 'Dr. Alice FOAF-Imported',
      organization: 'General Clinic',
      qualifications: const ['M.D.', 'Webizen Verified'],
      roles: const ['Primary Care'],
      verificationStatus: 'VERIFIED',
      pairwiseDid: 'did:qualia:pairwise:${DateTime.now().millisecondsSinceEpoch.toRadixString(16)}',
      rootDidUri: 'did:web:generalclinic.org:alice',
    );
    try {
      await api.addDirectoryActor(actor: actor);
      await _loadData();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
      }
    }
  }

  Future<void> _addDelegationRule(String actorId) async {
    final rule = api.DelegationRuleBridge(
      id: 'rule-${DateTime.now().millisecondsSinceEpoch}',
      actorId: actorId,
      grantedRoles: const ['read_clinical_records'],
      legalBasis: 'Explicit Consent',
      privacyModeLimit: 'MODE_B_PRIVILEGED',
      allowedRecordTypes: const ['PathologyObservation'],
      restrictedRecords: const ['PsychiatryObservation'],
      isActive: true,
    );
    try {
      await api.addDelegationRule(rule: rule);
      await _loadData();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
      }
    }
  }

  Future<void> _shareInvite() async {
    try {
      final invite = await api.generateFrontDoorInvite();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              invite.startsWith('{')
                  ? 'Connect invite JSON copied to snackbar — share via Profile for full code + email'
                  : 'Invite: $invite',
            ),
            duration: const Duration(seconds: 5),
          ),
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
    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Agreements & Consent',
            style: Theme.of(context).textTheme.headlineMedium?.copyWith(
              color: cs.primary,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 8),
          const Text(
            'Manage Front Door DIDs, verified actors, and delegation policies.',
            style: TextStyle(color: Colors.grey, fontSize: 13),
          ),
          const SizedBox(height: 16),
          Row(
            children: [
              _tabButton('Front Doors', 0, cs.primary),
              const SizedBox(width: 8),
              _tabButton('Actors', 1, const Color(0xFFB026FF)),
              const SizedBox(width: 8),
              _tabButton('Delegations', 2, const Color(0xFF00FF88)),
              const Spacer(),
              IconButton(icon: const Icon(Icons.refresh), onPressed: _loadData),
              IconButton(icon: const Icon(Icons.share), tooltip: 'Generate invite', onPressed: _shareInvite),
            ],
          ),
          const SizedBox(height: 16),
          Expanded(
            child: _loading
                ? const Center(child: CircularProgressIndicator())
                : _buildTabContent(cs),
          ),
        ],
      ),
    );
  }

  Widget _tabButton(String label, int index, Color color) {
    final active = _tab == index;
    return TextButton(
      onPressed: () => setState(() => _tab = index),
      style: TextButton.styleFrom(
        foregroundColor: active ? color : Colors.grey,
        side: BorderSide(color: active ? color.withOpacity(0.5) : Colors.white12),
      ),
      child: Text(label),
    );
  }

  Widget _buildTabContent(ColorScheme cs) {
    switch (_tab) {
      case 0:
        return ListView(
          children: [
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _doorLabelController,
                        decoration: const InputDecoration(
                          hintText: 'Label (e.g. Clinical Front Door)',
                          border: OutlineInputBorder(),
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    ElevatedButton(onPressed: _generateFrontDoor, child: const Text('Generate')),
                  ],
                ),
              ),
            ),
            ..._frontDoors.map((fd) => Card(
              child: ListTile(
                title: Text(fd.label),
                subtitle: Text(fd.didUri, style: TextStyle(color: cs.primary, fontFamily: 'monospace', fontSize: 11)),
                trailing: Text(fd.id, style: const TextStyle(color: Colors.grey, fontSize: 11)),
              ),
            )),
            if (_frontDoors.isEmpty)
              const Center(child: Padding(padding: EdgeInsets.all(32), child: Text('No Front Doors yet.'))),
          ],
        );
      case 1:
        return ListView(
          children: [
            Align(
              alignment: Alignment.centerRight,
              child: ElevatedButton.icon(
                onPressed: _syncFoaf,
                icon: const Icon(Icons.sync),
                label: const Text('Sync Solid FOAF'),
              ),
            ),
            const SizedBox(height: 12),
            ..._actors.map((a) => Card(
              child: ExpansionTile(
                title: Text(a.name),
                subtitle: Text('${a.actorType} • ${a.verificationStatus}'),
                children: [
                  Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('Pairwise: ${a.pairwiseDid}', style: const TextStyle(fontFamily: 'monospace', fontSize: 11)),
                        if (a.rootDidUri != null) Text('Root: ${a.rootDidUri}'),
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 6,
                          children: a.roles.map((r) => Chip(label: Text(r, style: const TextStyle(fontSize: 11)))).toList(),
                        ),
                        const SizedBox(height: 8),
                        OutlinedButton(
                          onPressed: () => _addDelegationRule(a.id),
                          child: const Text('Add delegation rule'),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            )),
            if (_actors.isEmpty)
              const Center(child: Padding(padding: EdgeInsets.all(32), child: Text('No actors in directory.'))),
          ],
        );
      default:
        return ListView(
          children: _rules.map((r) => Card(
            child: ListTile(
              leading: Icon(r.isActive ? Icons.check_circle : Icons.pause_circle, color: r.isActive ? Colors.green : Colors.grey),
              title: Text('Actor: ${r.actorId}'),
              subtitle: Text('${r.legalBasis} • ${r.grantedRoles.join(", ")}'),
            ),
          )).toList(),
        );
    }
  }
}
