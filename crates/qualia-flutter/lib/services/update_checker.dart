import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';

import '../src/rust/api/qualia_api.dart' as api;

/// Checks releases feed and can download + launch the platform installer.
class UpdateChecker {
  static const _feedUrl =
      'https://mediaprophet.github.io/qualiaDB/releases/latest.json';

  static Future<void> checkAndNotify(BuildContext context) async {
    try {
      final manifest = await api.fetchRemoteManifest(url: _feedUrl);
      final json = jsonDecode(manifest) as Map<String, dynamic>;
      final remoteVersion = json['version'] as String? ?? json['tag'] as String?;
      if (remoteVersion == null) return;

      final info = await PackageInfo.fromPlatform();
      final current = info.version;
      final remoteClean = remoteVersion.replaceFirst(RegExp(r'^v'), '');

      if (!_isNewer(remoteClean, current) || !context.mounted) return;

      final downloadUrl = _resolveDownloadUrl(json);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Update available: v$remoteClean (current v$current)'),
          action: SnackBarAction(
            label: 'Install',
            onPressed: () => _promptInstall(context, remoteClean, downloadUrl),
          ),
          duration: const Duration(seconds: 12),
        ),
      );
    } catch (e) {
      debugPrint('Update check skipped: $e');
    }
  }

  static String _resolveDownloadUrl(Map<String, dynamic> json) {
    const keys = ['windows_url', 'url', 'download_url', 'installer_url'];
    for (final k in keys) {
      final v = json[k];
      if (v is String && v.isNotEmpty) return v;
    }
    final platforms = json['platforms'];
    if (platforms is Map) {
      for (final key in ['windows-x86_64', 'windows', 'win64']) {
        final entry = platforms[key];
        if (entry is Map) {
          final url = entry['url'] ?? entry['download_url'];
          if (url is String && url.isNotEmpty) return url;
        }
      }
    }
    return 'https://github.com/mediaprophet/qualiaDB/releases/latest';
  }

  static Future<void> _promptInstall(
    BuildContext context,
    String version,
    String downloadUrl,
  ) async {
    final proceed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Install v$version?'),
        content: Text(
          'QualiaDB will download the installer and launch it. '
          'The app may close during installation.\n\n$downloadUrl',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          ElevatedButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Download & Install')),
        ],
      ),
    );
    if (proceed != true || !context.mounted) return;

    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => const AlertDialog(
        content: Row(
          children: [
            CircularProgressIndicator(),
            SizedBox(width: 16),
            Expanded(child: Text('Downloading update…')),
          ],
        ),
      ),
    );

    try {
      await api.downloadAndInstallUpdate(url: downloadUrl);
      if (context.mounted) {
        Navigator.pop(context);
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Installer launched — follow on-screen prompts.')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        Navigator.pop(context);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Update failed: $e')),
        );
      }
    }
  }

  static bool _isNewer(String remote, String current) {
    List<int> parse(String v) =>
        v.split('.').map((p) => int.tryParse(p) ?? 0).toList();
    final r = parse(remote);
    final c = parse(current);
    for (var i = 0; i < 3; i++) {
      final rv = i < r.length ? r[i] : 0;
      final cv = i < c.length ? c[i] : 0;
      if (rv > cv) return true;
      if (rv < cv) return false;
    }
    return false;
  }
}
