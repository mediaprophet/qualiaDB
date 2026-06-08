import 'dart:io';

import 'package:flutter/material.dart';

import '../src/rust/api/chat_files.dart' as chat_files;
import 'sensitivity_badge.dart';

/// Inline image preview for a chat-attached image file.
class ChatImageAttachment extends StatelessWidget {
  final String sessionId;
  final chat_files.ChatFileRecord file;
  final double maxHeight;

  const ChatImageAttachment({
    super.key,
    required this.sessionId,
    required this.file,
    this.maxHeight = 220,
  });

  @override
  Widget build(BuildContext context) {
    final sensitivityStyle =
        resolveSensitivityStyleFromLevel(context, file.sensitivityLevel);

    return FutureBuilder<String>(
      future: chat_files.getChatFileLocalPath(
        sessionId: sessionId,
        fileId: file.fileId,
        variant: file.thumbnailRelPath != null ? 'thumbnail' : 'original',
      ),
      builder: (context, snap) {
        if (!snap.hasData) {
          return Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: SizedBox(
              height: 80,
              child: Center(
                child: Icon(Icons.image_outlined,
                    color: Theme.of(context).colorScheme.primary),
              ),
            ),
          );
        }
        final path = snap.data!;
        if (!File(path).existsSync()) {
          return const SizedBox.shrink();
        }
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Container(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: sensitivityStyle.border),
              color: sensitivityStyle.background,
            ),
            child: Stack(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: ConstrainedBox(
                    constraints: BoxConstraints(maxHeight: maxHeight),
                    child: Image.file(
                      File(path),
                      fit: BoxFit.cover,
                      errorBuilder: (_, __, ___) =>
                          const Icon(Icons.broken_image_outlined),
                    ),
                  ),
                ),
                Positioned(
                  top: 8,
                  left: 8,
                  child: SensitivityBadge(
                    sensitivityLevel: file.sensitivityLevel,
                    dense: true,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
