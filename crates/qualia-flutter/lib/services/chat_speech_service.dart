import 'dart:async';

import 'package:flutter_tts/flutter_tts.dart';
import 'package:speech_to_text/speech_to_text.dart';

/// Voice input + TTS parity with qualia-client-old webizen.js.
class ChatSpeechService {
  ChatSpeechService._();
  static final ChatSpeechService instance = ChatSpeechService._();

  final SpeechToText _stt = SpeechToText();
  final FlutterTts _tts = FlutterTts();
  bool _sttReady = false;
  bool isListening = false;
  Timer? _ttsDebounce;

  Future<bool> init() async {
    _sttReady = await _stt.initialize();
    await _tts.setLanguage('en-US');
    await _tts.setSpeechRate(0.95);
    return _sttReady;
  }

  Future<void> toggleListening({
    required void Function(String text) onResult,
    void Function(String error)? onError,
  }) async {
    if (!_sttReady) {
      onError?.call('Speech recognition unavailable on this platform');
      return;
    }
    if (isListening) {
      await _stt.stop();
      isListening = false;
      return;
    }
    isListening = true;
    await _stt.listen(
      onResult: (r) {
        if (r.finalResult) {
          isListening = false;
          onResult(r.recognizedWords);
        }
      },
      listenMode: ListenMode.confirmation,
    );
  }

  void speakAgentResponse(String text) {
    if (text.trim().isEmpty) return;
    _ttsDebounce?.cancel();
    _ttsDebounce = Timer(const Duration(milliseconds: 800), () async {
      await _tts.stop();
      await _tts.speak(text);
    });
  }

  Future<void> stopSpeaking() => _tts.stop();

  void dispose() {
    _ttsDebounce?.cancel();
    _stt.stop();
    _tts.stop();
  }
}
