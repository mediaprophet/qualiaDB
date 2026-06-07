import 'package:flutter/material.dart';
import '../src/rust/api/qualia_api.dart' as api;

class ChatScreen extends StatefulWidget {
  /// Path to the active GGUF model file. Passed from the parent navigator
  /// (usually set after the user selects a model in LLMHubScreen).
  final String modelPath;

  const ChatScreen({super.key, this.modelPath = ''});

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final TextEditingController _promptController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  final List<_Message> _messages = [];
  bool _isInferring = false;

  @override
  void dispose() {
    _promptController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _sendMessage() async {
    final text = _promptController.text.trim();
    if (text.isEmpty || _isInferring) return;

    setState(() {
      _messages.add(_Message(role: 'user', content: text));
      _isInferring = true;
    });
    _promptController.clear();
    _scrollToBottom();

    try {
      // Call the Webizen-gated inference pipeline via FRB:
      //   validate_intent → Phase 8 SPSC GPU loop → validate_output
      final response = await api.runInference(
        prompt: text,
        modelPath: widget.modelPath,
      );
      setState(() {
        _messages.add(_Message(role: 'agent', content: response));
      });
    } catch (e) {
      setState(() {
        _messages.add(_Message(role: 'agent', content: '[Error: $e]'));
      });
    } finally {
      setState(() => _isInferring = false);
      _scrollToBottom();
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      children: [
        if (widget.modelPath.isEmpty)
          MaterialBanner(
            content: const Text('No model loaded — select one in LLM Hub'),
            actions: [
              TextButton(
                onPressed: () {},
                child: const Text('Dismiss'),
              ),
            ],
          ),
        Expanded(
          child: ListView.builder(
            controller: _scrollController,
            padding: const EdgeInsets.all(16),
            itemCount: _messages.length,
            itemBuilder: (context, i) {
              final m = _messages[i];
              final isUser = m.role == 'user';
              return Align(
                alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
                child: Container(
                  margin: const EdgeInsets.symmetric(vertical: 4),
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  constraints: BoxConstraints(
                    maxWidth: MediaQuery.of(context).size.width * 0.75,
                  ),
                  decoration: BoxDecoration(
                    color: isUser ? cs.primary.withOpacity(0.18) : cs.surfaceVariant,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(m.content),
                ),
              );
            },
          ),
        ),
        if (_isInferring)
          const LinearProgressIndicator(),
        Container(
          padding: const EdgeInsets.all(8),
          color: cs.surface,
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _promptController,
                  enabled: !_isInferring,
                  decoration: const InputDecoration(
                    hintText: 'Enter prompt…',
                    border: OutlineInputBorder(),
                  ),
                  onSubmitted: (_) => _sendMessage(),
                ),
              ),
              const SizedBox(width: 8),
              IconButton(
                icon: _isInferring
                    ? const SizedBox(
                        width: 20, height: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.send),
                color: cs.primary,
                onPressed: _isInferring ? null : _sendMessage,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _Message {
  final String role;    // 'user' | 'agent'
  final String content;
  const _Message({required this.role, required this.content});
}
