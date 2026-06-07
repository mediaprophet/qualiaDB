import 'dart:async';
import 'dart:convert';

import 'package:file_picker/file_picker.dart';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import 'qualia_qapp_webview.dart';
import '../services/chat_speech_service.dart';
import '../services/qpu_feature_service.dart';
import '../src/rust/api/qapp_api.dart';
import '../src/rust/api/qualia_api.dart' as api;
import '../src/rust/api/qualia_api_extras.dart' as api_extras;
import '../src/rust/api/resource_catalog.dart' as catalog;
import '../widgets/latex_math_keyboard.dart';
import '../widgets/markdown_message.dart';

class ChatScreen extends ConsumerStatefulWidget {
  final String modelPath;

  const ChatScreen({super.key, this.modelPath = ''});

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen> {

  final TextEditingController _promptController = TextEditingController();

  final ScrollController _scrollController = ScrollController();

  final List<_Message> _messages = [];

  bool _isInferring = false;

  bool _speechReady = false;

  bool _isListening = false;

  bool _ttsEnabled = true;
  bool _showMathKeyboard = false;

  StreamSubscription<String>? _streamSub;



  @override

  void initState() {

    super.initState();

    ChatSpeechService.instance.init().then((ok) {

      if (mounted) setState(() => _speechReady = ok);

    });

  }



  @override

  void dispose() {

    _streamSub?.cancel();

    _promptController.dispose();

    _scrollController.dispose();

    super.dispose();

  }



  Future<void> _ingestFile() async {

    final result = await FilePicker.platform.pickFiles(

      type: FileType.custom,

      allowedExtensions: ['txt', 'md', 'pdf'],

      dialogTitle: 'Ingest literature for context',

    );

    if (result == null || result.files.single.path == null) return;

    final path = result.files.single.path!;

    setState(() {

      _messages.add(_Message(role: 'user', content: '[Ingest] $path'));

      _messages.add(const _Message(role: 'agent', content: 'Ingesting…'));

      _isInferring = true;

    });

    final idx = _messages.length - 1;

    try {

      final summary = path.toLowerCase().endsWith('.pdf')

          ? (await api.ingestPdf(fileName: path))

          : await api.ingestLiterature(filePath: path);

      setState(() {

        _messages[idx] = _Message(

          role: 'agent',

          content: '⚡ Webizen verified ingest complete.\n\n$summary\n\n**Math ready:** \$\$ E = mc^2 \$\$',

        );

      });

    } catch (e) {

      setState(() {

        _messages[idx] = _Message(role: 'agent', content: '🔴 WEBIZEN BLOCKED: $e');

      });

    } finally {

      if (mounted) setState(() => _isInferring = false);

    }

  }



  _Message? _latestMessageForRole(String role) {
    for (final message in _messages.reversed) {
      if (message.role == role && message.content.trim().isNotEmpty) {
        return message;
      }
    }
    return null;
  }

  List<String> _extractAnatomyTargets(String text) {
    final normalized = text.toLowerCase();
    final targets = <String>[];
    final organMatchers = <String, List<String>>{
      'Brain (Allen)': ['brain', 'cortex', 'cerebral', 'neural', 'neuron'],
      'Heart': ['heart', 'cardiac', 'atrium', 'ventricle'],
      'Lung': ['lung', 'pulmonary', 'alveoli', 'bronch'],
      'Liver': ['liver', 'hepatic'],
      'Pancreas': ['pancreas', 'pancreatic'],
      'Spleen': ['spleen', 'splenic'],
      'Kidney (Left)': ['kidney', 'renal', 'neph'],
      'Small Intestine': ['intestine', 'bowel', 'gut', 'intestinal'],
      'Large Intestine': ['large intestine', 'colon', 'colonic'],
      'Skin': ['skin', 'integument', 'dermis', 'epiderm'],
      'Pelvis': ['pelvis', 'pelvic', 'hip bone'],
      'Knee (Left)': ['knee', 'patella', 'femur', 'tibia'],
      'Blood Vasculature': ['vasculature', 'artery', 'vein', 'capillar'],
      'Eye (Left)': ['eye', 'retina', 'vision', 'ocular'],
      'Spinal Cord': ['spinal cord', 'spine', 'vertebra'],
      'Thymus': ['thymus'],
      'Lymph Node': ['lymph node', 'lymphatic'],
      'Urinary Bladder': ['bladder', 'urinary bladder'],
      'Prostate': ['prostate'],
      'Uterus': ['uterus', 'uterine'],
      'Ovary (Left)': ['ovary', 'ovarian'],
    };

    for (final entry in organMatchers.entries) {
      if (entry.value.any(normalized.contains)) {
        targets.add(entry.key);
      }
    }

    return targets;
  }

  List<String> _inferAnatomySystems(String text) {
    final normalized = text.toLowerCase();
    final matches = <String>[];
    final systemMatchers = <String, List<String>>{
      'Circulatory (Cardiovascular) System': [
        'heart',
        'cardiac',
        'artery',
        'vein',
        'blood',
        'circulat',
      ],
      'Respiratory System': [
        'lung',
        'respirat',
        'bronch',
        'trachea',
        'oxygen',
        'carbon dioxide',
      ],
      'Digestive System': [
        'digest',
        'stomach',
        'liver',
        'pancrea',
        'intestin',
        'gut',
      ],
      'Nervous System': [
        'brain',
        'neural',
        'nerve',
        'spinal',
        'cognition',
        'autonomic',
      ],
      'Muscular System': [
        'muscle',
        'posture',
        'skeletal muscle',
        'smooth muscle',
      ],
      'Skeletal System': [
        'bone',
        'skeletal',
        'cartilage',
        'ligament',
        'tendon',
      ],
      'Endocrine System': [
        'hormone',
        'pituitary',
        'thyroid',
        'adrenal',
        'metabolism',
      ],
      'Immune / Lymphatic System': [
        'immune',
        'lymph',
        'spleen',
        'thymus',
        'pathogen',
      ],
      'Integumentary System': [
        'skin',
        'hair',
        'nail',
        'sweat',
        'sebaceous',
      ],
      'Urinary (Excretory) System': [
        'kidney',
        'renal',
        'bladder',
        'ureter',
        'urinary',
      ],
      'Reproductive System': [
        'uterus',
        'ovary',
        'prostate',
        'testes',
        'reproductive',
      ],
      'Sensory System': [
        'sensory',
        'vision',
        'visual',
        'hearing',
        'auditory',
        'touch',
        'taste',
        'smell',
        'olfact',
        'somatosensory',
      ],
      'Vestibular System': [
        'vestibular',
        'balance',
        'vertigo',
        'dizziness',
        'otolith',
        'semicircular',
        'inner ear',
        'spatial orientation',
      ],
      'Exocrine System': [
        'exocrine',
        'salivary',
        'mammary',
        'mucous gland',
        'duct secretion',
        'sweat gland',
      ],
      'Endocannabinoid System (ECS)': [
        'endocannabinoid',
        'cannabinoid',
        'cb1',
        'cb2',
        'anandamide',
        '2-ag',
        'homeostasis signaling',
      ],
      'Enteric Nervous System (ENS)': [
        'enteric',
        'gut brain',
        'second brain',
        'gastrointestinal nerve',
        'myenteric',
        'submucosal plexus',
        'gi motility',
      ],
      'Glymphatic System': [
        'glymphatic',
        'csf clearance',
        'cerebrospinal fluid',
        'amyloid clearance',
        'brain waste',
        'perivascular',
        'astrocyte',
      ],
    };

    for (final entry in systemMatchers.entries) {
      if (entry.value.any(normalized.contains)) {
        matches.add(entry.key);
      }
    }

    return matches;
  }

  String? _inferReferenceSex(String text) {
    final normalized = text.toLowerCase();
    if (normalized.contains('uterus') ||
        normalized.contains('ovary') ||
        normalized.contains('ovarian')) {
      return 'female';
    }
    if (normalized.contains('prostate')) {
      return 'male';
    }
    return null;
  }

  Future<void> _openLatestInAnatomy({String? dicomFilePath}) async {
    final latestAgent = _latestMessageForRole('agent');
    if (latestAgent == null) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Ask the local model something first.')),
        );
      }
      return;
    }

    final latestUser = _latestMessageForRole('user');
    final userPrompt = latestUser?.content ?? '';
    final agentReply = latestAgent.content;

    final targets = _extractAnatomyTargets(agentReply);
    var systems = _inferAnatomySystems(agentReply);
    final sex = _inferReferenceSex(agentReply);

    final conditions = <String>[];
    final conditionImpactMap = <String, String>{};
    Map<String, dynamic>? dicomOverlay;
    var graphSource = 'chat-derived';
    var daemonReachable = false;
    var daemonMatchCount = 0;

    try {
      final graphJson = await api_extras.buildAnatomyGraphContext(
        qappName: 'Anatomy',
        userPrompt: userPrompt,
        agentReply: agentReply,
        dicomFilePath: dicomFilePath,
      );
      final graph = jsonDecode(graphJson) as Map<String, dynamic>;
      graphSource = graph['source'] as String? ?? graphSource;
      daemonReachable = graph['daemonReachable'] as bool? ?? false;
      daemonMatchCount = (graph['daemonMatchCount'] as num?)?.toInt() ?? 0;

      for (final item in graph['conditions'] as List<dynamic>? ?? const []) {
        conditions.add(item.toString());
      }
      for (final entry
          in (graph['conditionImpactMap'] as Map<String, dynamic>? ?? const {})
              .entries) {
        conditionImpactMap[entry.key] = entry.value.toString();
      }
      for (final item in graph['systems'] as List<dynamic>? ?? const []) {
        systems.add(item.toString());
      }
      systems = systems.toSet().toList();
      final overlay = graph['dicomOverlay'];
      if (overlay is Map<String, dynamic>) {
        dicomOverlay = overlay;
      }
    } catch (e) {
      debugPrint('Anatomy graph context fallback: $e');
    }

    var focusOrgan = targets.isNotEmpty ? targets.first : null;
    if (dicomOverlay?['organ'] is String) {
      focusOrgan = dicomOverlay!['organ'] as String;
    }

    BigInt? dicomIngestJobId;
    if (dicomFilePath != null && dicomFilePath.isNotEmpty) {
      try {
        final patientHash = qappIdHash(appId: 'did:qualia:patient:local');
        dicomIngestJobId =
            submitDicomIngest(filePath: dicomFilePath, patientDidHash: patientHash);
      } catch (e) {
        debugPrint('DICOM Core-3 ingest skipped: $e');
      }
    }

    final payload = <String, Object?>{
      'version': '1.0.0',
      'intent': 'anatomy:represent-graph-complexity',
      'source': 'qualia.flutter.chat',
      'surface': 'panel',
      'prompt': userPrompt.isEmpty ? null : userPrompt,
      'summary': agentReply,
      'conditions': conditions,
      if (conditionImpactMap.isNotEmpty) 'conditionImpactMap': conditionImpactMap,
      'graphSummary': {
        'representation': 'anatomy-model',
        'complexityMode': graphSource,
        'activeModelPath': widget.modelPath,
        'conditions': conditions,
        'daemonReachable': daemonReachable,
        'daemonMatchCount': daemonMatchCount,
      },
      'anatomySelection': {
        'sex': sex,
        'focusOrgan': focusOrgan,
        'candidateOrgans': targets,
        'systemFocus': systems.isNotEmpty ? systems.first : null,
        'relatedSystems': systems,
      },
      if (dicomOverlay != null) 'dicomOverlay': dicomOverlay,
      if (dicomIngestJobId != null) 'dicomIngestJobId': dicomIngestJobId.toString(),
      if (dicomFilePath != null) 'dicomFilePath': dicomFilePath,
      'explanationCard': {
        'title': 'Local Qualia chat handoff',
        'body': agentReply,
      },
      'systemOverlay': {
        'focusSystem': systems.isNotEmpty ? systems.first : null,
        'relatedSystems': systems,
      },
    };

    try {
      try {
        registerQappFromInstalledManifest(qappName: 'Anatomy');
      } catch (_) {}
      final url = await api_extras.launchInstalledQappWithContext(
        qappName: 'Anatomy',
        entrypoint: 'representation',
        surface: 'panel',
        payloadJson: jsonEncode(payload),
        source: 'qualia.flutter.chat',
      );

      if (!mounted) return;

      if (url.startsWith('http://127.0.0.1') || url.startsWith('http://localhost')) {
        await Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => QualiaQappWebView(
              url: url,
              title: focusOrgan != null ? 'Anatomy: $focusOrgan' : 'Anatomy',
            ),
          ),
        );
      } else {
        final uri = Uri.parse(url);
        if (!await launchUrl(uri, mode: LaunchMode.externalApplication)) {
          throw Exception('Could not open $url');
        }
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not open Anatomy: $e')),
        );
      }
    }
  }

  Future<void> _openLatestInAnatomyWithDicom() async {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: const ['dcm'],
      allowMultiple: false,
      withData: false,
    );
    if (result == null || result.files.isEmpty) return;

    final path = result.files.single.path;
    if (path == null || path.isEmpty) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Could not read the selected DICOM path.')),
        );
      }
      return;
    }

    await _openLatestInAnatomy(dicomFilePath: path);
  }

  Future<void> _toggleMic() async {

    await ChatSpeechService.instance.toggleListening(

      onResult: (text) {

        if (!mounted) return;

        setState(() {

          _isListening = false;

          _promptController.text = text;

        });

        _sendMessage();

      },

      onError: (e) {

        if (mounted) {

          ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(e)));

          setState(() => _isListening = false);

        }

      },

    );

    setState(() => _isListening = ChatSpeechService.instance.isListening);

  }



  Future<void> _sendMessage() async {

    final text = _promptController.text.trim();

    if (text.isEmpty || _isInferring) return;



    setState(() {

      _messages.add(_Message(role: 'user', content: text));

      _messages.add(const _Message(role: 'agent', content: ''));

      _isInferring = true;

    });

    _promptController.clear();

    _scrollToBottom();



    final agentIndex = _messages.length - 1;

    try {
      final qpuCmd = await api.handleEngineChatCommand(text: text);
      if (qpuCmd.handled) {
        ref.read(qpuFeatureUnlockedProvider.notifier).setUnlocked(qpuCmd.featureUnlocked);
        setState(() {
          _messages[agentIndex] = _Message(role: 'agent', content: qpuCmd.response);
          _isInferring = false;
        });
        _scrollToBottom();
        return;
      }

      final lifecycleJson = await catalog.getModelLifecycleStatus();
      final lifecycle = _parseLifecycleState(lifecycleJson);
      if (lifecycle != 'Active') {
        setState(() {
          _messages[agentIndex] = _Message(
            role: 'agent',
            content:
                'No active model — download and activate a model in LLM Hub first.',
          );
          _isInferring = false;
        });
        _scrollToBottom();
        return;
      }

      _streamSub?.cancel();
      final stream = api.runInferenceStream(

        prompt: text,

        modelPath: widget.modelPath,

      );

      _streamSub = stream.listen(

        (token) {

          setState(() {

            _messages[agentIndex] = _Message(

              role: 'agent',

              content: _messages[agentIndex].content + token,

            );

          });

          _scrollToBottom();

        },

        onError: (e) {

          setState(() {

            _messages[agentIndex] = _Message(role: 'agent', content: '[Error: $e]');

          });

        },

        onDone: () {

          if (!mounted) return;

          setState(() => _isInferring = false);

          if (_ttsEnabled) {

            ChatSpeechService.instance.speakAgentResponse(_messages[agentIndex].content);

          }

        },

      );

    } catch (e) {

      setState(() {

        _messages[agentIndex] = _Message(role: 'agent', content: '[Error: $e]');

        _isInferring = false;

      });

    }

  }



  String _parseLifecycleState(String lifecycleJson) {
    try {
      final map = jsonDecode(lifecycleJson) as Map<String, dynamic>;
      return map['lifecycle_state'] as String? ?? '';
    } catch (_) {
      return '';
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

            content: const Text(
              'No model loaded — download and activate a model in LLM Hub',
            ),

            actions: [

              TextButton(onPressed: () {}, child: const Text('Dismiss')),

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

              final display = m.content.isEmpty && _isInferring && !isUser ? '…' : m.content;

              return Align(

                alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,

                child: Container(

                  margin: const EdgeInsets.symmetric(vertical: 4),

                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),

                  constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.75),

                  decoration: BoxDecoration(

                    color: isUser ? cs.primary.withValues(alpha: 0.18) : cs.surfaceContainerHighest,

                    borderRadius: BorderRadius.circular(12),

                  ),

                  child: isUser
                      ? (display.contains(r'$')
                          ? MarkdownMessage(content: display, style: TextStyle(color: cs.onSurface))
                          : Text(display))
                      : MarkdownMessage(content: display, style: TextStyle(color: cs.onSurface)),

                ),

              );

            },

          ),

        ),

        if (_isInferring) const LinearProgressIndicator(),
        if (_showMathKeyboard)
          LatexMathKeyboard(
            onInsert: (s) => insertAtCursor(_promptController, s),
            onClose: () => setState(() => _showMathKeyboard = false),
          ),
        Container(
          padding: const EdgeInsets.all(8),
          color: cs.surface,
          child: Row(
            children: [
              IconButton(
                icon: Icon(_showMathKeyboard ? Icons.functions : Icons.calculate_outlined),
                color: _showMathKeyboard ? cs.primary : cs.secondary,
                tooltip: 'Math / LaTeX keyboard',
                onPressed: _isInferring
                    ? null
                    : () => setState(() => _showMathKeyboard = !_showMathKeyboard),
              ),
              IconButton(
                icon: Icon(_isListening ? Icons.mic : Icons.mic_none),

                color: _isListening ? Colors.redAccent : cs.secondary,

                tooltip: _speechReady ? 'Voice input' : 'Speech unavailable',

                onPressed: _speechReady && !_isInferring ? _toggleMic : null,

              ),

              IconButton(

                icon: Icon(_ttsEnabled ? Icons.volume_up : Icons.volume_off),

                tooltip: 'Toggle TTS',

                onPressed: () {

                  setState(() => _ttsEnabled = !_ttsEnabled);

                  if (!_ttsEnabled) ChatSpeechService.instance.stopSpeaking();

                },

              ),

              IconButton(

                icon: const Icon(Icons.upload_file),

                tooltip: 'Ingest file',

                onPressed: _isInferring ? null : _ingestFile,

              ),

              IconButton(

                icon: const Icon(Icons.biotech_outlined),

                tooltip: 'Open latest reply in Anatomy',

                onPressed: _isInferring ? null : () => _openLatestInAnatomy(),

              ),

              IconButton(

                icon: const Icon(Icons.medical_services_outlined),

                tooltip: 'Open in Anatomy with DICOM overlay',

                onPressed: _isInferring ? null : _openLatestInAnatomyWithDicom,

              ),

              Expanded(

                child: TextField(

                  controller: _promptController,

                  enabled: !_isInferring,

                  decoration: const InputDecoration(
                    hintText: 'Prompt, LaTeX (\$\$…\$\$), or [qpu:qubo]…',
                    border: OutlineInputBorder(),
                  ),
                  maxLines: _showMathKeyboard ? 3 : 1,

                  onSubmitted: (_) => _sendMessage(),

                ),

              ),

              const SizedBox(width: 8),

              IconButton(

                icon: _isInferring

                    ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))

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

  final String role;

  final String content;

  const _Message({required this.role, required this.content});

}

