import 'package:flutter/material.dart';
import 'package:webview_flutter/webview_flutter.dart';

/// In-app sandbox for qualia:// / loopback qapp assets.
class QualiaQappWebView extends StatefulWidget {
  final String url;
  final String title;
  final VoidCallback? onClose;

  const QualiaQappWebView({
    super.key,
    required this.url,
    required this.title,
    this.onClose,
  });

  @override
  State<QualiaQappWebView> createState() => _QualiaQappWebViewState();
}

class _QualiaQappWebViewState extends State<QualiaQappWebView> {
  late final WebViewController _controller;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(
        NavigationDelegate(
          onPageFinished: (_) {
            if (mounted) setState(() => _loading = false);
          },
        ),
      )
      ..loadRequest(Uri.parse(widget.url));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
        leading: IconButton(
          icon: const Icon(Icons.close),
          tooltip: 'Close',
          onPressed: widget.onClose ?? () => Navigator.of(context).pop(),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => _controller.reload(),
          ),
        ],
      ),
      body: Stack(
        children: [
          WebViewWidget(controller: _controller),
          if (_loading) const LinearProgressIndicator(),
        ],
      ),
    );
  }
}
