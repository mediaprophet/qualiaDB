import 'dart:io';

import 'package:flutter/material.dart';

import '../src/rust/api/chat_files.dart' as files;

/// Configure sharing permissions before attaching a file to a chat session.
class ChatFilePermissionsSheet extends StatefulWidget {
  final String fileName;
  final String? sourcePath;
  final files.ChatFilePreview? preview;
  final files.ChatFileSharing initialSharing;
  final List<String> participantDids;

  const ChatFilePermissionsSheet({
    super.key,
    required this.fileName,
    this.sourcePath,
    required this.preview,
    required this.initialSharing,
    this.participantDids = const [],
  });

  @override
  State<ChatFilePermissionsSheet> createState() =>
      _ChatFilePermissionsSheetState();
}

class _ChatFilePermissionsSheetState extends State<ChatFilePermissionsSheet> {
  late String _visibility;
  late bool _allowDownload;
  late bool _allowLlmContext;
  late bool _allowRelaySync;
  final _allowedController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _visibility = widget.initialSharing.visibility;
    _allowDownload = widget.initialSharing.allowDownload;
    _allowLlmContext = widget.initialSharing.allowLlmContext;
    _allowRelaySync = widget.initialSharing.allowRelaySync;
    if (widget.initialSharing.allowedDids.isNotEmpty) {
      _allowedController.text = widget.initialSharing.allowedDids.join(', ');
    }
  }

  @override
  void dispose() {
    _allowedController.dispose();
    super.dispose();
  }

  bool _isImagePath(String name) {
    final lower = name.toLowerCase();
    return lower.endsWith('.png') ||
        lower.endsWith('.jpg') ||
        lower.endsWith('.jpeg') ||
        lower.endsWith('.webp') ||
        lower.endsWith('.gif');
  }

  files.ChatFileSharing _buildSharing() {
    final allowed = _allowedController.text
        .split(RegExp(r'[,\s]+'))
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    return files.ChatFileSharing(
      visibility: _visibility,
      allowDownload: _allowDownload,
      allowLlmContext: _allowLlmContext,
      allowRelaySync: _allowRelaySync,
      allowedDids: allowed,
      expiresAt: widget.initialSharing.expiresAt,
    );
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final preview = widget.preview;

    return Padding(
      padding: EdgeInsets.only(
        left: 16,
        right: 16,
        top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Attach file',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 8),
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(
                widget.fileName.toLowerCase().endsWith('.pdf')
                    ? Icons.picture_as_pdf
                    : Icons.description_outlined,
                color: cs.primary,
              ),
              title: Text(widget.fileName, maxLines: 2, overflow: TextOverflow.ellipsis),
              subtitle: preview == null
                  ? null
                  : Text(
                      '${preview.parseStatus}'
                      '${preview.pageCount != null ? ' · ${preview.pageCount} pages' : ''}',
                    ),
            ),
            if (widget.sourcePath != null &&
                _isImagePath(widget.fileName) &&
                File(widget.sourcePath!).existsSync()) ...[
              ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 160),
                  child: Image.file(
                    File(widget.sourcePath!),
                    fit: BoxFit.cover,
                  ),
                ),
              ),
              const SizedBox(height: 12),
            ] else if (preview != null && preview.textPreview.isNotEmpty) ...[
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: cs.surfaceContainerHighest.withValues(alpha: 0.5),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Text(
                  preview.textPreview,
                  maxLines: 4,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              const SizedBox(height: 12),
            ],
            Text('Who can access this file?', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            DropdownButtonFormField<String>(
              initialValue: _visibility,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                isDense: true,
              ),
              items: const [
                DropdownMenuItem(
                  value: 'owner_only',
                  child: Text('Only me'),
                ),
                DropdownMenuItem(
                  value: 'session_participants',
                  child: Text('All chat participants'),
                ),
                DropdownMenuItem(
                  value: 'specific_dids',
                  child: Text('Specific people (DIDs)'),
                ),
                DropdownMenuItem(
                  value: 'public_in_session',
                  child: Text('Anyone with session access'),
                ),
              ],
              onChanged: (v) => setState(() => _visibility = v ?? _visibility),
            ),
            if (_visibility == 'specific_dids') ...[
              const SizedBox(height: 8),
              TextField(
                controller: _allowedController,
                decoration: InputDecoration(
                  labelText: 'Allowed DIDs',
                  hintText: widget.participantDids.isEmpty
                      ? 'did:qualia:…, did:qualia:…'
                      : widget.participantDids.join(', '),
                  border: const OutlineInputBorder(),
                  isDense: true,
                ),
                maxLines: 2,
              ),
            ],
            const SizedBox(height: 12),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Allow download'),
              subtitle: const Text('Participants can save a copy'),
              value: _allowDownload,
              onChanged: (v) => setState(() => _allowDownload = v),
            ),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Include in LLM context'),
              subtitle: const Text('Agent can read extracted text when replying'),
              value: _allowLlmContext,
              onChanged: (v) => setState(() => _allowLlmContext = v),
            ),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Sync via relay'),
              subtitle: const Text('Share metadata with group relay (when available)'),
              value: _allowRelaySync,
              onChanged: (v) => setState(() => _allowRelaySync = v),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: () => Navigator.pop(context),
                    child: const Text('Cancel'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: FilledButton.icon(
                    onPressed: () => Navigator.pop(context, _buildSharing()),
                    icon: const Icon(Icons.attach_file),
                    label: const Text('Attach'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
