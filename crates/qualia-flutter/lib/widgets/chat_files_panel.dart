import 'package:flutter/material.dart';

import '../src/rust/api/chat_files.dart' as files;
import 'chat_image_attachment.dart';
import 'sensitivity_badge.dart';

/// Lists files attached to the current chat session.
class ChatFilesPanel extends StatelessWidget {
  final List<files.ChatFileRecord> chatFiles;
  final String? sessionId;
  final String? ownerDid;
  final void Function(files.ChatFileRecord file)? onEditSharing;

  const ChatFilesPanel({
    super.key,
    required this.chatFiles,
    this.sessionId,
    this.ownerDid,
    this.onEditSharing,
  });

  bool _isImage(files.ChatFileRecord file) {
    return file.mediaKind == 'image' ||
        file.extension_ == 'png' ||
        file.extension_ == 'jpg' ||
        file.extension_ == 'jpeg' ||
        file.extension_ == 'webp' ||
        file.extension_ == 'gif';
  }

  IconData _iconFor(files.ChatFileRecord file) {
    if (_isImage(file)) return Icons.image_outlined;
    if (file.extension_ == 'pdf') return Icons.picture_as_pdf;
    if (file.extension_ == 'md') return Icons.article_outlined;
    return Icons.description_outlined;
  }

  String _visibilityLabel(String v) {
    switch (v) {
      case 'owner_only':
        return 'Only me';
      case 'session_participants':
        return 'Participants';
      case 'specific_dids':
        return 'Specific DIDs';
      case 'public_in_session':
        return 'Session access';
      default:
        return v;
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    if (chatFiles.isEmpty) {
      return Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          'No files attached yet. Use the attach button to add PDF or text files.',
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: cs.onSurfaceVariant,
              ),
        ),
      );
    }

    return ListView.separated(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      padding: const EdgeInsets.symmetric(vertical: 8),
      itemCount: chatFiles.length,
      separatorBuilder: (_, __) => const SizedBox(height: 2),
      itemBuilder: (context, i) {
        final file = chatFiles[i];
        final isOwner = ownerDid != null && file.authorDid == ownerDid;
        final sensitivityStyle =
            resolveSensitivityStyleFromLevel(context, file.sensitivityLevel);

        return Container(
          margin: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            color: sensitivityStyle.background,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: sensitivityStyle.border),
          ),
          child: ListTile(
            dense: true,
            leading: _isImage(file) && sessionId != null
                ? SizedBox(
                    width: 44,
                    height: 44,
                    child: ChatImageAttachment(
                      sessionId: sessionId!,
                      file: file,
                      maxHeight: 44,
                    ),
                  )
                : Icon(_iconFor(file), color: sensitivityStyle.foreground),
            title:
                Text(file.originalName, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 4),
                SensitivityBadge(
                  sensitivityLevel: file.sensitivityLevel,
                  dense: true,
                ),
                const SizedBox(height: 6),
                Text(
                  '${_visibilityLabel(file.sharing.visibility)}'
                  '${file.pageCount != null ? ' · ${file.pageCount} pg' : ''}'
                  ' · ${file.parseStatus}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                if (file.textPreview.isNotEmpty)
                  Text(
                    file.textPreview,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: cs.onSurfaceVariant,
                        ),
                  ),
              ],
            ),
            trailing: isOwner && onEditSharing != null
                ? IconButton(
                    icon: const Icon(Icons.lock_outline, size: 20),
                    tooltip: 'Sharing permissions',
                    onPressed: () => onEditSharing!(file),
                  )
                : null,
          ),
        );
      },
    );
  }
}
