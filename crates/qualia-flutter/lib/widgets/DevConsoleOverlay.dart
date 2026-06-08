import 'dart:async';
import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../rust/frb_generated.dart';

/// DevConsoleOverlay wraps a [child] with a developer console that can be
/// toggled via the `~` key or a hidden triple-tap gesture. When visible it
/// streams raw telemetry logs coming from the Rust engine.
class DevConsoleOverlay extends StatefulWidget {
  const DevConsoleOverlay({super.key, required this.child, this.maxLogEntries = 400});

  final Widget child;
  final int maxLogEntries;

  @override
  State<DevConsoleOverlay> createState() => _DevConsoleOverlayState();
}

class _DevConsoleOverlayState extends State<DevConsoleOverlay> {
  final FocusNode _focusNode = FocusNode(debugLabel: 'DevConsoleOverlayFocus');
  final ScrollController _scrollController = ScrollController();
  final ListQueue<String> _logBuffer = ListQueue();
  StreamSubscription<String>? _logSubscription;

  bool _advancedMode = false;
  bool _pendingAutoScroll = false;
  DateTime? _lastTap;
  int _tapCount = 0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && !_focusNode.hasFocus) {
        _focusNode.requestFocus();
      }
    });
  }

  @override
  void dispose() {
    _logSubscription?.cancel();
    _scrollController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _toggleAdvancedMode() {
    setState(() {
      _advancedMode = !_advancedMode;
    });
    if (_advancedMode) {
      _ensureTelemetryStream();
    }
  }

  void _ensureTelemetryStream() {
    if (_logSubscription != null) {
      return;
    }

    try {
      final stream = RustApi.instance.api.crateApiQualiaApiInitTelemetryStream();
      _logSubscription = stream.listen(
        _handleLogLine,
        onError: (error, stack) {
          _handleLogLine('[error] DevConsoleOverlay: $error');
        },
        cancelOnError: false,
      );
    } catch (err, stack) {
      _handleLogLine('[error] DevConsoleOverlay: $err');
      debugPrintStack(label: 'DevConsoleOverlay telemetry subscription error', stackTrace: stack);
    }
  }

  void _handleLogLine(String line) {
    setState(() {
      _logBuffer.addLast(line);
      while (_logBuffer.length > widget.maxLogEntries) {
        _logBuffer.removeFirst();
      }
      _pendingAutoScroll = true;
    });

    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_pendingAutoScroll || !_scrollController.hasClients) return;
      _pendingAutoScroll = false;
      _scrollController.animateTo(
        _scrollController.position.maxScrollExtent,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      );
    });
  }

  KeyEventResult _handleKey(RawKeyEvent event) {
    if (event is RawKeyDownEvent) {
      if (event.logicalKey == LogicalKeyboardKey.backquote) {
        _toggleAdvancedMode();
        return KeyEventResult.handled;
      }
    }
    return KeyEventResult.ignored;
  }

  void _handleTripleTap() {
    final now = DateTime.now();
    if (_lastTap == null || now.difference(_lastTap!) > const Duration(milliseconds: 800)) {
      _tapCount = 0;
    }
    _tapCount += 1;
    _lastTap = now;

    if (_tapCount >= 3) {
      _tapCount = 0;
      _toggleAdvancedMode();
    }
  }

  Widget _buildConsolePanel(ThemeData theme) {
    return AnimatedSlide(
      duration: const Duration(milliseconds: 240),
      curve: Curves.easeOutQuad,
      offset: _advancedMode ? Offset.zero : const Offset(0, 1),
      child: AnimatedOpacity(
        duration: const Duration(milliseconds: 180),
        opacity: _advancedMode ? 1 : 0,
        child: Container(
          height: MediaQuery.of(context).size.height * 0.4,
          width: double.infinity,
          decoration: BoxDecoration(
            color: theme.colorScheme.surface.withOpacity(0.94),
            border: Border(
              top: BorderSide(color: theme.colorScheme.primary.withOpacity(0.35), width: 1),
            ),
            boxShadow: const [
              BoxShadow(blurRadius: 24, offset: Offset(0, -8), color: Colors.black38),
            ],
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                child: Row(
                  children: [
                    const Icon(Icons.terminal, size: 18),
                    const SizedBox(width: 8),
                    const Text(
                      'Rust Telemetry Stream',
                      style: TextStyle(fontWeight: FontWeight.w600),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        '${_logBuffer.length} entries',
                        style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.outline),
                      ),
                    ),
                    FilledButton.tonal(
                      onPressed: _toggleAdvancedMode,
                      child: const Text('Exit Advanced Mode'),
                    ),
                  ],
                ),
              ),
              const Divider(height: 1),
              Expanded(
                child: Container(
                  color: Colors.black.withOpacity(0.82),
                  child: ListView.builder(
                    controller: _scrollController,
                    itemCount: _logBuffer.length,
                    itemBuilder: (context, index) {
                      final text = _logBuffer.elementAt(index);
                      return Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                        child: Text(
                          text,
                          style: const TextStyle(
                            fontFamily: 'RobotoMono',
                            fontSize: 12,
                            color: Colors.greenAccent,
                          ),
                        ),
                      );
                    },
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return RawKeyboardListener(
      focusNode: _focusNode,
      onKey: _handleKey,
      autofocus: true,
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: _handleTripleTap,
        child: Stack(
          children: [
            widget.child,
            Positioned.fill(
              child: Align(
                alignment: Alignment.bottomCenter,
                child: _buildConsolePanel(theme),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
