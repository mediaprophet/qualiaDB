import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';

import '../src/rust/api/qualia_api.dart' as api;

/// First-launch gate for Windows runtime dependencies (WebView2 + VC++ redist).
///
/// WebView2 is expected to ship with the app (Fixed Version under
/// `WebView2Runtime/`). If missing, the user can install the Evergreen
/// bootstrapper. VC++ must be installed by the user; we download the official
/// installer and re-detect once it completes.
class PrerequisitesOverlay extends StatefulWidget {
  final VoidCallback onComplete;

  const PrerequisitesOverlay({super.key, required this.onComplete});

  @override
  State<PrerequisitesOverlay> createState() => _PrerequisitesOverlayState();
}

class _PrerequisitesOverlayState extends State<PrerequisitesOverlay> {
  api.PrerequisiteStatus? _status;
  String? _error;
  String? _busyKind;
  Timer? _poll;

  @override
  void initState() {
    super.initState();
    _refresh();
    _poll = Timer.periodic(const Duration(seconds: 3), (_) => _refresh(silent: true));
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  Future<void> _refresh({bool silent = false}) async {
    if (!Platform.isWindows) {
      widget.onComplete();
      return;
    }
    try {
      await api.configureWebview2Runtime();
      final status = await api.checkPrerequisites();
      if (!mounted) return;
      setState(() {
        _status = status;
        if (!silent) _error = null;
      });
      if (status.allReady) {
        _poll?.cancel();
        widget.onComplete();
      }
    } catch (e) {
      if (mounted && !silent) setState(() => _error = '$e');
    }
  }

  Future<void> _install(String kind) async {
    setState(() {
      _busyKind = kind;
      _error = null;
    });
    try {
      await api.installPrerequisite(kind: kind);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              kind == 'vc_redist'
                  ? 'VC++ installer launched — complete it, then QualiaDB will detect it automatically.'
                  : 'WebView2 installer launched — follow the prompts, then QualiaDB will detect it.',
            ),
            duration: const Duration(seconds: 8),
          ),
        );
      }
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busyKind = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final status = _status;
    return Material(
      color: Colors.black.withValues(alpha: 0.94),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Card(
            margin: const EdgeInsets.all(24),
            child: Padding(
              padding: const EdgeInsets.all(28),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    'Runtime setup',
                    style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'QualiaDB needs two Windows components before WebViews and native '
                    'libraries can run. WebView2 is bundled with the installer when '
                    'available; Visual C++ must be installed once on your machine.',
                    style: TextStyle(color: Colors.grey, height: 1.35),
                  ),
                  const SizedBox(height: 24),
                  if (status == null)
                    const Center(child: CircularProgressIndicator())
                  else ...[
                    _DependencyRow(
                      title: 'Microsoft WebView2',
                      subtitle: _webviewSubtitle(status),
                      ready: status.webview2Ready,
                      actionLabel: status.webview2Ready
                          ? null
                          : (status.webview2Bundled
                              ? 'Repair install'
                              : 'Install WebView2'),
                      busy: _busyKind == 'webview2',
                      onAction: status.webview2Ready
                          ? null
                          : () => _install('webview2'),
                    ),
                    const SizedBox(height: 16),
                    _DependencyRow(
                      title: 'Visual C++ 2015–2022 (x64)',
                      subtitle: status.vcRedistReady
                          ? 'Installed — required by QualiaDB native libraries.'
                          : 'Not detected — download and run Microsoft\'s redistributable.',
                      ready: status.vcRedistReady,
                      actionLabel: status.vcRedistReady ? null : 'Download & install',
                      busy: _busyKind == 'vc_redist',
                      onAction:
                          status.vcRedistReady ? null : () => _install('vc_redist'),
                    ),
                  ],
                  if (_error != null) ...[
                    const SizedBox(height: 16),
                    Text(_error!, style: const TextStyle(color: Colors.redAccent, fontSize: 12)),
                  ],
                  const SizedBox(height: 20),
                  OutlinedButton.icon(
                    onPressed: status == null ? null : () => _refresh(),
                    icon: const Icon(Icons.refresh, size: 18),
                    label: const Text('Recheck now'),
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Installers run separately — this screen updates automatically when each component is detected.',
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

  String _webviewSubtitle(api.PrerequisiteStatus status) {
    if (status.webview2Ready && status.webview2Bundled) {
      return 'Bundled runtime found next to QualiaDB.';
    }
    if (status.webview2Ready && status.webview2Evergreen) {
      return 'System WebView2 Evergreen runtime detected.';
    }
    if (status.webview2Bundled && status.bundledWebview2Dir.isEmpty) {
      return 'Bundled folder expected but incomplete — reinstall or install WebView2.';
    }
    return 'Required for Qapp Vault, spatial mesh, and in-qapp web views.';
  }
}

class _DependencyRow extends StatelessWidget {
  final String title;
  final String subtitle;
  final bool ready;
  final String? actionLabel;
  final bool busy;
  final VoidCallback? onAction;

  const _DependencyRow({
    required this.title,
    required this.subtitle,
    required this.ready,
    this.actionLabel,
    this.busy = false,
    this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    final color = ready ? Colors.greenAccent : Colors.orangeAccent;
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        border: Border.all(color: color.withValues(alpha: 0.35)),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(ready ? Icons.check_circle : Icons.warning_amber_rounded, color: color, size: 20),
              const SizedBox(width: 8),
              Expanded(
                child: Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(subtitle, style: const TextStyle(fontSize: 12, color: Colors.grey, height: 1.3)),
          if (actionLabel != null && onAction != null) ...[
            const SizedBox(height: 10),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonal(
                onPressed: busy ? null : onAction,
                child: busy
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : Text(actionLabel!),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
