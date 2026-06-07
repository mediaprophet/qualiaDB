import 'dart:async';

import 'package:app_links/app_links.dart';

/// Parsed `qualia://` deep link routes (OS handler + in-app navigation).
class QualiaDeepLink {
  final String route;
  final String? qappName;
  final String? assetPath;

  const QualiaDeepLink({
    required this.route,
    this.qappName,
    this.assetPath,
  });
}

class DeepLinkService {
  DeepLinkService._();
  static final DeepLinkService instance = DeepLinkService._();

  final AppLinks _appLinks = AppLinks();
  StreamSubscription<Uri>? _sub;

  /// qualia://settings | qualia://chat | qualia://qapp/{name} | qualia://localhost/{qapp}/...
  static QualiaDeepLink? parse(Uri uri) {
    if (uri.scheme != 'qualia') return null;
    final host = uri.host.toLowerCase();
    final path = uri.path.replaceFirst(RegExp(r'^/'), '');

    switch (host) {
      case 'settings':
        return const QualiaDeepLink(route: 'settings');
      case 'chat':
        return const QualiaDeepLink(route: 'chat');
      case 'wallet':
        return const QualiaDeepLink(route: 'wallet');
      case 'qapp':
        if (path.isNotEmpty) {
          return QualiaDeepLink(route: 'qapp', qappName: path.split('/').first);
        }
        return null;
      case 'localhost':
        if (path.isNotEmpty) {
          final parts = path.split('/');
          return QualiaDeepLink(route: 'qapp', qappName: parts.first, assetPath: path);
        }
        return null;
      default:
        return null;
    }
  }

  Future<QualiaDeepLink?> getInitialLink() async {
    final uri = await _appLinks.getInitialLink();
    if (uri == null) return null;
    return parse(uri);
  }

  void listen(void Function(QualiaDeepLink link) onLink) {
    _sub?.cancel();
    _sub = _appLinks.uriLinkStream.listen((uri) {
      final link = parse(uri);
      if (link != null) onLink(link);
    });
  }

  void dispose() => _sub?.cancel();
}
