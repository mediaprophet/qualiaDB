import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../src/rust/api/social_api.dart' as social;

/// Local identity, sharing policy, and friend connect invites.
class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key});

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  final _nameController = TextEditingController();
  final _bioController = TextEditingController();
  final _inviteInputController = TextEditingController();
  final _relayUrlController = TextEditingController();

  social.UserProfile? _profile;
  social.ConnectInviteSummary? _invite;
  bool _loading = true;
  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _nameController.dispose();
    _bioController.dispose();
    _inviteInputController.dispose();
    _relayUrlController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final profile = await social.getUserProfile();
      if (!mounted) return;
      _nameController.text = profile.displayName;
      _bioController.text = profile.bio ?? '';
      _relayUrlController.text = profile.relayBaseUrl ?? '';
      setState(() {
        _profile = profile;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<void> _save() async {
    final base = _profile;
    if (base == null) return;
    setState(() => _saving = true);
    try {
      final updated = social.UserProfile(
        displayName: _nameController.text.trim().isEmpty
            ? 'Qualia User'
            : _nameController.text.trim(),
        bio: _bioController.text.trim().isEmpty ? null : _bioController.text.trim(),
        publicDid: base.publicDid,
        activeFrontDoorId: base.activeFrontDoorId,
        relayBaseUrl: _relayUrlController.text.trim().isEmpty
            ? null
            : _relayUrlController.text.trim(),
        sharing: base.sharing,
        updatedAt: base.updatedAt,
      );
      final saved = await social.saveUserProfile(profile: updated);
      if (!mounted) return;
      setState(() {
        _profile = saved;
        _saving = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Profile saved')),
      );
    } catch (e) {
      if (!mounted) return;
      setState(() => _saving = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Save failed: $e')),
      );
    }
  }

  void _updateSharing(social.SharingPolicy sharing) {
    final base = _profile;
    if (base == null) return;
    setState(() {
      _profile = social.UserProfile(
        displayName: base.displayName,
        bio: base.bio,
        publicDid: base.publicDid,
        activeFrontDoorId: base.activeFrontDoorId,
        sharing: sharing,
        updatedAt: base.updatedAt,
      );
    });
  }

  Future<void> _generateInvite() async {
    try {
      final invite = await social.generateConnectInvite(frontDoorId: null);
      if (!mounted) return;
      setState(() => _invite = invite);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Invite failed: $e')),
      );
    }
  }

  Future<void> _copyInvite() async {
    final invite = _invite;
    if (invite == null) return;
    await Clipboard.setData(ClipboardData(text: invite.inviteJson));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Invite JSON copied')),
    );
  }

  Future<void> _emailInvite() async {
    final invite = _invite;
    if (invite == null || invite.mailtoUrl.isEmpty) return;
    final uri = Uri.parse(invite.mailtoUrl);
    if (!await launchUrl(uri)) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Could not open email client')),
      );
    }
  }

  Future<void> _acceptInvite() async {
    final input = _inviteInputController.text.trim();
    if (input.isEmpty) return;
    try {
      final contact = await social.acceptConnectInvite(input: input);
      _inviteInputController.clear();
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Added ${contact.displayName}')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Accept failed: $e')),
      );
    }
  }

  Widget _sharingSwitch({
    required String title,
    required String subtitle,
    required bool value,
    required ValueChanged<bool> onChanged,
  }) {
    return SwitchListTile(
      title: Text(title),
      subtitle: Text(subtitle, style: Theme.of(context).textTheme.bodySmall),
      value: value,
      onChanged: onChanged,
    );
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    if (_loading) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }

    if (_error != null) {
      return Scaffold(
        body: Center(child: Text(_error!, style: TextStyle(color: cs.error))),
      );
    }

    final profile = _profile!;
    final sharing = profile.sharing;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Profile'),
        actions: [
          if (_saving)
            const Padding(
              padding: EdgeInsets.all(16),
              child: SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2)),
            )
          else
            TextButton(onPressed: _save, child: const Text('Save')),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          TextField(
            controller: _nameController,
            decoration: const InputDecoration(
              labelText: 'Display name',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _bioController,
            decoration: const InputDecoration(
              labelText: 'Bio (optional)',
              border: OutlineInputBorder(),
            ),
            maxLines: 2,
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _relayUrlController,
            decoration: const InputDecoration(
              labelText: 'Relay URL (for group chat sync)',
              hintText: 'http://192.168.1.10:4242',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          SelectableText(
            profile.publicDid.isEmpty ? 'Resolving public DID…' : profile.publicDid,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  color: cs.secondary,
                ),
          ),
          const SizedBox(height: 24),
          Text('Sharing in chats & invites', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Card(
            child: Column(
              children: [
                _sharingSwitch(
                  title: 'Display name',
                  subtitle: 'Show your name in group chats and invites',
                  value: sharing.shareDisplayName,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: v,
                    sharePublicDid: sharing.sharePublicDid,
                    shareActiveModel: sharing.shareActiveModel,
                    shareOntologyScope: sharing.shareOntologyScope,
                    shareInstalledQapps: sharing.shareInstalledQapps,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: sharing.allowGroupChatInvites,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: sharing.allowEmailInvites,
                  )),
                ),
                _sharingSwitch(
                  title: 'Public DID',
                  subtitle: 'Share your Front Door / root DID for routing',
                  value: sharing.sharePublicDid,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: sharing.shareDisplayName,
                    sharePublicDid: v,
                    shareActiveModel: sharing.shareActiveModel,
                    shareOntologyScope: sharing.shareOntologyScope,
                    shareInstalledQapps: sharing.shareInstalledQapps,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: sharing.allowGroupChatInvites,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: sharing.allowEmailInvites,
                  )),
                ),
                _sharingSwitch(
                  title: 'Active model',
                  subtitle: 'Let group context include your LLM model info',
                  value: sharing.shareActiveModel,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: sharing.shareDisplayName,
                    sharePublicDid: sharing.sharePublicDid,
                    shareActiveModel: v,
                    shareOntologyScope: sharing.shareOntologyScope,
                    shareInstalledQapps: sharing.shareInstalledQapps,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: sharing.allowGroupChatInvites,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: sharing.allowEmailInvites,
                  )),
                ),
                _sharingSwitch(
                  title: 'Ontology scope',
                  subtitle: 'Share which ontologies ground your answers',
                  value: sharing.shareOntologyScope,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: sharing.shareDisplayName,
                    sharePublicDid: sharing.sharePublicDid,
                    shareActiveModel: sharing.shareActiveModel,
                    shareOntologyScope: v,
                    shareInstalledQapps: sharing.shareInstalledQapps,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: sharing.allowGroupChatInvites,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: sharing.allowEmailInvites,
                  )),
                ),
                _sharingSwitch(
                  title: 'Installed qapps',
                  subtitle: 'Share qapp names available on this device',
                  value: sharing.shareInstalledQapps,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: sharing.shareDisplayName,
                    sharePublicDid: sharing.sharePublicDid,
                    shareActiveModel: sharing.shareActiveModel,
                    shareOntologyScope: sharing.shareOntologyScope,
                    shareInstalledQapps: v,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: sharing.allowGroupChatInvites,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: sharing.allowEmailInvites,
                  )),
                ),
                _sharingSwitch(
                  title: 'Group chat invites',
                  subtitle: 'Allow others to connect via your invite codes',
                  value: sharing.allowGroupChatInvites,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: sharing.shareDisplayName,
                    sharePublicDid: sharing.sharePublicDid,
                    shareActiveModel: sharing.shareActiveModel,
                    shareOntologyScope: sharing.shareOntologyScope,
                    shareInstalledQapps: sharing.shareInstalledQapps,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: v,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: sharing.allowEmailInvites,
                  )),
                ),
                _sharingSwitch(
                  title: 'Email invites',
                  subtitle: 'Include mailto link when sharing connect codes',
                  value: sharing.allowEmailInvites,
                  onChanged: (v) => _updateSharing(social.SharingPolicy(
                    shareDisplayName: sharing.shareDisplayName,
                    sharePublicDid: sharing.sharePublicDid,
                    shareActiveModel: sharing.shareActiveModel,
                    shareOntologyScope: sharing.shareOntologyScope,
                    shareInstalledQapps: sharing.shareInstalledQapps,
                    shareDaemonStatus: sharing.shareDaemonStatus,
                    allowGroupChatInvites: sharing.allowGroupChatInvites,
                    allowDirectoryLookup: sharing.allowDirectoryLookup,
                    allowEmailInvites: v,
                  )),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          Text('Connect with friends', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Row(
            children: [
              FilledButton.icon(
                onPressed: _generateInvite,
                icon: const Icon(Icons.qr_code_2),
                label: const Text('Generate connect code'),
              ),
              if (_invite != null) ...[
                const SizedBox(width: 8),
                OutlinedButton(onPressed: _copyInvite, child: const Text('Copy JSON')),
                if (_invite!.mailtoUrl.isNotEmpty) ...[
                  const SizedBox(width: 8),
                  OutlinedButton(onPressed: _emailInvite, child: const Text('Email')),
                ],
              ],
            ],
          ),
          if (_invite != null) ...[
            const SizedBox(height: 12),
            Text('Code: ${_invite!.code}', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text(
              'Share the JSON (or email link). Friends paste it below.',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
          const SizedBox(height: 16),
          TextField(
            controller: _inviteInputController,
            decoration: const InputDecoration(
              labelText: 'Paste friend invite JSON',
              border: OutlineInputBorder(),
            ),
            maxLines: 4,
          ),
          const SizedBox(height: 8),
          FilledButton(
            onPressed: _acceptInvite,
            child: const Text('Add friend'),
          ),
        ],
      ),
    );
  }
}
