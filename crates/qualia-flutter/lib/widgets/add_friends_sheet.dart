import 'package:flutter/material.dart';

import '../src/rust/api/chat_session.dart' as chat;
import '../src/rust/api/social_api.dart' as social;

/// Pick friends to start a group chat or add to an existing session.
class AddFriendsSheet extends StatefulWidget {
  final String? sessionId;
  final bool createGroup;
  final ValueChanged<String>? onGroupCreated;
  final VoidCallback? onParticipantsChanged;

  const AddFriendsSheet({
    super.key,
    this.sessionId,
    this.createGroup = false,
    this.onGroupCreated,
    this.onParticipantsChanged,
  });

  @override
  State<AddFriendsSheet> createState() => _AddFriendsSheetState();
}

class _AddFriendsSheetState extends State<AddFriendsSheet> {
  List<social.ChatContact> _contacts = [];
  final Set<String> _selected = {};
  bool _loading = true;
  String? _error;
  bool _working = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final contacts = await social.listChatContacts();
      if (!mounted) return;
      setState(() {
        _contacts = contacts;
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

  Future<void> _editCategories(social.ChatContact contact) async {
    final controller = TextEditingController(text: contact.categories.join(', '));
    final saved = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Categories for ${contact.displayName}'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            labelText: 'Sharing categories',
            hintText: 'clinical, research, family',
            border: OutlineInputBorder(),
          ),
          maxLines: 2,
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Save')),
        ],
      ),
    );
    if (saved != true) return;
    try {
      final cats = controller.text
          .split(RegExp(r'[,\s]+'))
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toList();
      await social.updateChatContactCategories(
        contactDid: contact.did,
        categories: cats,
      );
      await _load();
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Contact categories updated')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Update failed: $e')),
      );
    }
  }

  Future<void> _confirm() async {
    if (_selected.isEmpty) return;
    setState(() => _working = true);
    try {
      if (widget.createGroup) {
        final id = await chat.createGroupChatSession(
          title: null,
          participantDids: _selected.toList(),
        );
        widget.onGroupCreated?.call(id);
      } else if (widget.sessionId != null) {
        for (final did in _selected) {
          await chat.addChatParticipant(
            sessionId: widget.sessionId!,
            participantDid: did,
          );
        }
        widget.onParticipantsChanged?.call();
      }
      if (!mounted) return;
      Navigator.pop(context);
    } catch (e) {
      if (!mounted) return;
      setState(() => _working = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Failed: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: EdgeInsets.only(
        left: 16,
        right: 16,
        top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            widget.createGroup ? 'New group chat' : 'Add to chat',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          if (_loading)
            const Padding(
              padding: EdgeInsets.all(24),
              child: Center(child: CircularProgressIndicator()),
            )
          else if (_error != null)
            Text(_error!, style: TextStyle(color: cs.error))
          else if (_contacts.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 16),
              child: Column(
                children: [
                  const Text('No friends yet.'),
                  const SizedBox(height: 8),
                  Text(
                    'Open Profile to generate a connect code or accept a friend invite.',
                    style: Theme.of(context).textTheme.bodySmall,
                    textAlign: TextAlign.center,
                  ),
                ],
              ),
            )
          else
            ConstrainedBox(
              constraints: BoxConstraints(
                maxHeight: MediaQuery.of(context).size.height * 0.45,
              ),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: _contacts.length,
                itemBuilder: (context, i) {
                  final c = _contacts[i];
                  final checked = _selected.contains(c.did);
                  final catLabel = c.categories.isEmpty
                      ? 'no categories'
                      : c.categories.join(', ');
                  return CheckboxListTile(
                    value: checked,
                    onChanged: (v) {
                      setState(() {
                        if (v == true) {
                          _selected.add(c.did);
                        } else {
                          _selected.remove(c.did);
                        }
                      });
                    },
                    secondary: IconButton(
                      icon: const Icon(Icons.label_outline, size: 20),
                      tooltip: 'Edit sharing categories',
                      onPressed: () => _editCategories(c),
                    ),
                    title: Text(c.displayName),
                    subtitle: Text(
                      '$catLabel\n${c.did}',
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  );
                },
              ),
            ),
          const SizedBox(height: 12),
          FilledButton(
            onPressed: _working || _selected.isEmpty ? null : _confirm,
            child: _working
                ? const SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text(widget.createGroup ? 'Start group' : 'Add selected'),
          ),
        ],
      ),
    );
  }
}
