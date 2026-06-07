import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../screens/qualia_qapp_webview.dart';
import '../src/rust/api/qualia_api.dart';

/// Launches installed qapps: browser for vault/standalone, in-app panel from chat.
class QappLauncher {
  QappLauncher._();

  static Future<void> ensureProtocol() async {
    try {
      await startQualiaProtocol();
    } catch (_) {
      // init_core already starts the loopback server; ignore races.
    }
  }

  /// Open a resolved qapp URL in the system default browser (Chrome, Edge, etc.).
  static Future<void> openInBrowser(String url) async {
    final uri = Uri.parse(url);
    final launched = await launchUrl(uri, mode: LaunchMode.externalApplication);
    if (!launched) {
      throw Exception('Could not open $url in the default browser');
    }
  }

  /// Vault, deep links, and standalone launches — always external browser.
  static Future<void> launchInstalledToBrowser({required String qappName}) async {
    await ensureProtocol();
    final url = await launchInstalledQapp(qappName: qappName);
    await openInBrowser(url);
  }

  /// Chat handoff — slide-over panel with explicit close/back to chat.
  static Future<void> showPanel(
    BuildContext context, {
    required String url,
    required String title,
  }) {
    return showGeneralDialog<void>(
      context: context,
      barrierDismissible: true,
      barrierLabel: 'Close $title',
      transitionDuration: const Duration(milliseconds: 220),
      pageBuilder: (dialogContext, _, __) {
        final size = MediaQuery.sizeOf(dialogContext);
        final panelWidth = (size.width * 0.82).clamp(360.0, 1200.0);
        final panelHeight = (size.height * 0.9).clamp(400.0, size.height);

        return SafeArea(
          child: Stack(
            children: [
              Positioned.fill(
                child: GestureDetector(
                  onTap: () => Navigator.of(dialogContext).pop(),
                  child: Container(color: Colors.black54),
                ),
              ),
              Align(
                alignment: Alignment.centerRight,
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(8, 8, 12, 8),
                  child: Material(
                    elevation: 12,
                    borderRadius: BorderRadius.circular(12),
                    clipBehavior: Clip.antiAlias,
                    child: SizedBox(
                      width: panelWidth,
                      height: panelHeight,
                      child: QualiaQappWebView(
                        url: url,
                        title: title,
                        onClose: () => Navigator.of(dialogContext).pop(),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        );
      },
      transitionBuilder: (context, animation, _, child) {
        final slide = Tween<Offset>(
          begin: const Offset(0.08, 0),
          end: Offset.zero,
        ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOutCubic));
        return FadeTransition(
          opacity: animation,
          child: SlideTransition(position: slide, child: child),
        );
      },
    );
  }
}
