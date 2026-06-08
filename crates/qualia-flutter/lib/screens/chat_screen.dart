import 'dart:async';
import 'dart:convert';

import 'package:file_picker/file_picker.dart';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'chat_environment_sheet.dart';
import 'chat_history_drawer.dart';
import '../widgets/add_friends_sheet.dart';
import '../widgets/chat_agent_outcome_sheet.dart';
import '../widgets/chat_file_permissions_sheet.dart';
import '../widgets/chat_files_panel.dart';
import '../widgets/chat_image_attachment.dart';
import '../widgets/chat_graph_panel.dart';
import '../widgets/chat_reaction_bar.dart';
import '../widgets/chat_session_shares_sheet.dart';
import '../src/rust/api/chat_files.dart' as chat_files;
import '../src/rust/api/chat_graph.dart' as graph;
import '../src/rust/api/social_api.dart' as social;
import '../services/qapp_launcher.dart';
import '../services/chat_speech_service.dart';
import '../src/rust/api/chat_session.dart' as chat;
import '../services/qpu_feature_service.dart';
import '../src/rust/api/qapp_api.dart';
import '../main.dart' show activeModelPathProvider, shellNavIndexProvider;
import '../src/rust/api/qualia_api.dart' as api;
import '../src/rust/api/qualia_api_extras.dart' as api_extras;
import '../src/rust/api/resource_catalog.dart' as catalog;
import '../widgets/chat_citation_chips.dart';
import '../widgets/sensitivity_badge.dart';
import '../widgets/super_quin_provenance_chip.dart';
import '../widgets/shield_alert.dart';
import '../widgets/guardian_affirmation_chip.dart';
import '../services/pending_affirmations_service.dart';
import '../widgets/chat_environment_bar.dart';
import '../widgets/vault_hud_bar.dart';
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

  String? _sessionId;
  String _sessionTitle = 'Chat';
  List<chat.ChatParticipant> _participants = [];
  bool _isGroup = false;
  String? _replyToFragmentId;
  String? _replyAnchorPreview;
  _PendingSelection? _pendingSelection;
  List<graph.ChatBranchType> _branchTypes = [];
  String? _selectedBranchTypeId;
  Map<BigInt, List<graph.ChatReaction>> _reactions = {};
  List<chat_files.ChatFileRecord> _chatFiles = [];
  String? _ownerDid;
  Timer? _relayTimer;
  final GlobalKey<ScaffoldState> _scaffoldKey = GlobalKey<ScaffoldState>();

  bool _isInferring = false;
  bool _sessionLoading = true;

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

    _initSession();
    _relayTimer =
        Timer.periodic(const Duration(seconds: 4), (_) => _pullRelay());
    _loadBranchTypes();
    _loadOwnerDid();
    _syncActiveModelFromRust();
  }

  Future<void> _syncActiveModelFromRust({bool force = false}) async {
    try {
      final active = await api.getActiveModel();
      if (!mounted || active == null || active.isEmpty) return;
      if (force || ref.read(activeModelPathProvider).isEmpty) {
        ref.read(activeModelPathProvider.notifier).state = active;
      }
    } catch (_) {}
  }

  Future<bool> _ensureActiveModel() async {
    try {
      final lifecycleJson = await catalog.getModelLifecycleStatus();
      if (_parseLifecycleState(lifecycleJson) == 'Active') return true;
      await catalog.applyModelPreference(task: 'chat');
      await _syncActiveModelFromRust(force: true);
      final retry = await catalog.getModelLifecycleStatus();
      return _parseLifecycleState(retry) == 'Active';
    } catch (_) {
      return false;
    }
  }

  Future<void> _openEnvironmentSheet() async {
    if (_sessionId == null || _isInferring) return;
    final changed = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      builder: (_) => ChatEnvironmentSheet(sessionId: _sessionId!),
    );
    if (changed == true && mounted) {
      await _syncActiveModelFromRust(force: true);
      setState(() {});
    }
  }

  Future<void> _loadOwnerDid() async {
    try {
      final profile = await social.getUserProfile();
      if (mounted) setState(() => _ownerDid = profile.publicDid);
    } catch (_) {}
  }

  Future<void> _refreshGuardianStatuses() async {
    bool changed = false;
    for (var i = 0; i < _messages.length; i++) {
      final m = _messages[i];
      final agreementId = m.suspendedAgreementId;
      if (!m.walSuspended || agreementId == null || m.guardianRatified)
        continue;
      try {
        final ratified = await api.isAgreementRatified(
          agreementId: BigInt.from(agreementId),
        );
        if (ratified) {
          _messages[i] =
              m.copyWith(guardianRatified: true, walSuspended: false);
          changed = true;
        }
      } catch (_) {}
    }
    if (changed && mounted) setState(() {});
  }

  chat_files.ChatFileRecord? _fileForLamport(BigInt? lamport) {
    if (lamport == null) return null;
    for (final f in _chatFiles) {
      if (f.messageLamport == lamport) return f;
    }
    return null;
  }

  Future<void> _loadChatFiles() async {
    final id = _sessionId;
    if (id == null) return;
    try {
      final files = await chat_files.listChatFiles(sessionId: id);
      if (mounted) setState(() => _chatFiles = files);
    } catch (_) {}
  }

  Future<void> _loadBranchTypes() async {
    try {
      final types = await graph.listChatBranchTypes();
      if (mounted) setState(() => _branchTypes = types);
    } catch (_) {}
  }

  Future<void> _loadReactions() async {
    final id = _sessionId;
    if (id == null) return;
    try {
      final reactions = await graph.listChatReactions(sessionId: id);
      if (!mounted) return;
      final map = <BigInt, List<graph.ChatReaction>>{};
      for (final r in reactions) {
        map.putIfAbsent(r.messageLamport, () => []).add(r);
      }
      setState(() => _reactions = map);
    } catch (_) {}
  }

  Future<void> _pullRelay() async {
    if (_sessionId == null || !_isGroup) return;
    try {
      final n = await graph.syncChatRelay(sessionId: _sessionId);
      if (n > BigInt.zero && mounted) {
        await _reloadMessages();
      }
    } catch (_) {}
  }

  Future<void> _reloadMessages() async {
    final id = _sessionId;
    if (id == null) return;
    final stored = await chat.loadChatSessionMessages(id: id);
    if (!mounted) return;
    setState(() {
      _messages
        ..clear()
        ..addAll(stored.map((m) => _Message(
              role: m.role,
              content: m.content,
              lamport: m.lamport,
              authorName: m.authorDisplay ?? m.authorName,
              replyToFragment: m.replyToFragment,
              subAgentOf: m.subAgentOf,
              modelId: m.modelId,
            )));
    });
  }

  Future<void> _showAgentOutcomeSharing() async {
    final id = _sessionId;
    if (id == null) return;
    await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      builder: (_) => ChatAgentOutcomeSheet(
        sessionId: id,
        participantDids: _participants.map((p) => p.did).toList(),
      ),
    );
  }

  Future<void> _loadParticipants(String id) async {
    try {
      final participants = await chat.getChatParticipants(sessionId: id);
      if (!mounted) return;
      setState(() {
        _participants = participants;
        _isGroup = participants.length > 1;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _participants = [];
        _isGroup = false;
      });
    }
  }

  Future<void> _initSession() async {
    try {
      final id = await chat.ensureChatSession();
      final title = await chat.loadChatSessionTitle(id: id);
      final stored = await chat.loadChatSessionMessages(id: id);
      if (!mounted) return;
      setState(() {
        _sessionId = id;
        _sessionTitle = title;
        _messages
          ..clear()
          ..addAll(stored.map((m) => _Message(
                role: m.role,
                content: m.content,
                lamport: m.lamport,
                authorName: m.authorDisplay ?? m.authorName,
                replyToFragment: m.replyToFragment,
                subAgentOf: m.subAgentOf,
                modelId: m.modelId,
              )));
        _sessionLoading = false;
      });
      await _loadParticipants(id);
      await _pullRelay();
      await _loadReactions();
      await _loadChatFiles();
    } catch (e) {
      debugPrint('Chat session init failed: $e');
      if (mounted) setState(() => _sessionLoading = false);
    }
  }

  Future<void> _startNewChat() async {
    try {
      final id = await chat.createChatSession(title: 'New chat');
      if (!mounted) return;
      setState(() {
        _sessionId = id;
        _sessionTitle = 'New chat';
        _messages.clear();
        _participants = [];
        _isGroup = false;
      });
      await chat.setLastChatSessionId(sessionId: id);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not create chat: $e')),
        );
      }
    }
  }

  Future<void> _switchSession(String id) async {
    try {
      final title = await chat.loadChatSessionTitle(id: id);
      final stored = await chat.loadChatSessionMessages(id: id);
      await chat.setLastChatSessionId(sessionId: id);
      if (!mounted) return;
      setState(() {
        _sessionId = id;
        _sessionTitle = title;
        _messages
          ..clear()
          ..addAll(stored.map((m) => _Message(
                role: m.role,
                content: m.content,
                lamport: m.lamport,
                authorName: m.authorDisplay ?? m.authorName,
                replyToFragment: m.replyToFragment,
                subAgentOf: m.subAgentOf,
                modelId: m.modelId,
              )));
      });
      await _loadParticipants(id);
      await _pullRelay();
      await _loadReactions();
      await _loadChatFiles();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not load chat: $e')),
        );
      }
    }
  }

  Future<void> _openAddFriends({bool createGroup = false}) async {
    if (_sessionId == null && !createGroup) return;
    await showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => AddFriendsSheet(
        sessionId: _sessionId,
        createGroup: createGroup,
        onGroupCreated: (id) => _switchSession(id),
        onParticipantsChanged: () {
          if (_sessionId != null) _loadParticipants(_sessionId!);
        },
      ),
    );
  }

  Future<void> _persistMessage(
    String role,
    String content, {
    String? replyToFragment,
    String? branchTypeId,
  }) async {
    final id = _sessionId;
    if (id == null || content.trim().isEmpty) return;
    try {
      if (replyToFragment != null) {
        await graph.appendChatMessageReply(
          sessionId: id,
          role: role,
          content: content,
          replyToFragment: replyToFragment,
          branchTypeId: branchTypeId,
        );
      } else {
        await chat.appendChatMessage(
            sessionId: id, role: role, content: content);
      }
    } catch (e) {
      debugPrint('Failed to persist chat message: $e');
    }
  }

  void _onTextSelected(int messageIndex, BigInt lamport, String content,
      TextSelection selection) {
    if (selection.start == selection.end) return;
    final start = selection.start.clamp(0, content.length);
    final end = selection.end.clamp(start, content.length);
    final selected = content.substring(start, end).trim();
    if (selected.isEmpty) return;
    setState(() {
      _pendingSelection = _PendingSelection(
        messageIndex: messageIndex,
        lamport: lamport,
        content: content,
        start: start,
        end: end,
        text: selected,
      );
      _replyAnchorPreview = selected;
    });
  }

  Future<void> _prepareReplyFragment() async {
    final pending = _pendingSelection;
    final id = _sessionId;
    if (pending == null || id == null) return;
    try {
      final fragment = await graph.createChatFragment(
        sessionId: id,
        messageLamport: pending.lamport,
        anchorStart: pending.start,
        anchorEnd: pending.end,
      );
      if (!mounted) return;
      setState(() {
        _replyToFragmentId = fragment.fragmentId;
        _replyAnchorPreview = fragment.anchorText;
      });
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Fragment failed: $e')),
        );
      }
    }
  }

  String _branchTypeLabel(String? id) {
    if (id == null) return 'Type';
    for (final t in _branchTypes) {
      if (t.id == id) return '${t.emoji} ${t.label}';
    }
    return 'Type';
  }

  void _clearReplyTarget() {
    setState(() {
      _replyToFragmentId = null;
      _replyAnchorPreview = null;
      _pendingSelection = null;
    });
  }

  @override
  void dispose() {
    _streamSub?.cancel();
    _relayTimer?.cancel();

    _promptController.dispose();

    _scrollController.dispose();

    super.dispose();
  }

  Future<void> _attachChatFile() async {
    final sessionId = _sessionId;
    if (sessionId == null) return;

    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: [
        'txt',
        'md',
        'pdf',
        'png',
        'jpg',
        'jpeg',
        'webp',
        'gif'
      ],
      dialogTitle: 'Attach file or image to chat',
    );
    if (result == null || result.files.single.path == null) return;

    final path = result.files.single.path!;
    final fileName = result.files.single.name;

    chat_files.ChatFilePreview? preview;
    try {
      preview = await chat_files.previewChatFile(sourcePath: path);
    } catch (_) {}

    chat_files.ChatFileSharing sharing;
    try {
      sharing = await chat_files.defaultChatFileSharing(sessionId: sessionId);
    } catch (_) {
      sharing = chat_files.ChatFileSharing(
        visibility: _isGroup ? 'session_participants' : 'owner_only',
        allowDownload: true,
        allowLlmContext: true,
        allowRelaySync: false,
        sensitivityLevel: _isGroup ? 1 : 2,
        allowedDids: [],
        expiresAt: null,
      );
    }

    if (!mounted) return;
    final participantDids = _participants.map((p) => p.did).toList();
    final confirmed = await showModalBottomSheet<chat_files.ChatFileSharing>(
      context: context,
      isScrollControlled: true,
      builder: (_) => ChatFilePermissionsSheet(
        fileName: fileName,
        sourcePath: path,
        preview: preview,
        initialSharing: sharing,
        participantDids: participantDids,
      ),
    );
    if (confirmed == null) return;

    setState(() => _isInferring = true);
    try {
      final attached = await chat_files.attachChatFile(
        sessionId: sessionId,
        sourcePath: path,
        sharing: confirmed,
      );
      await _reloadMessages();
      await _loadChatFiles();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              'Attached ${attached.file.originalName}'
              '${attached.file.pageCount != null ? ' (${attached.file.pageCount} pages)' : ''}',
            ),
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Attach failed: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _isInferring = false);
    }
  }

  Future<void> _editFileSharing(chat_files.ChatFileRecord file) async {
    final sessionId = _sessionId;
    if (sessionId == null) return;

    final confirmed = await showModalBottomSheet<chat_files.ChatFileSharing>(
      context: context,
      isScrollControlled: true,
      builder: (_) => ChatFilePermissionsSheet(
        fileName: file.originalName,
        preview: chat_files.ChatFilePreview(
          mimeType: file.mimeType,
          extension_: file.extension_,
          pageCount: file.pageCount,
          textPreview: file.textPreview,
          parseStatus: file.parseStatus,
          parseError: file.parseError,
          mediaKind: file.mediaKind,
          imageWidth: file.imageWidth,
          imageHeight: file.imageHeight,
        ),
        initialSharing: chat_files.ChatFileSharing(
          visibility: file.sharing.visibility,
          allowDownload: file.sharing.allowDownload,
          allowLlmContext: file.sharing.allowLlmContext,
          allowRelaySync: file.sharing.allowRelaySync,
          sensitivityLevel: file.sensitivityLevel,
          allowedDids: file.sharing.allowedDids,
          expiresAt: file.sharing.expiresAt,
        ),
        participantDids: _participants.map((p) => p.did).toList(),
      ),
    );
    if (confirmed == null) return;

    try {
      await chat_files.setChatFileSharing(
        sessionId: sessionId,
        fileId: file.fileId,
        sharing: confirmed,
      );
      await _loadChatFiles();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not update sharing: $e')),
        );
      }
    }
  }

  void _showSessionShares() {
    final sessionId = _sessionId;
    if (sessionId == null) return;
    showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      builder: (_) => SizedBox(
        height: MediaQuery.of(context).size.height * 0.55,
        child: ChatSessionSharesSheet(sessionId: sessionId),
      ),
    ).then((posted) {
      if (posted == true && mounted) _reloadMessages();
    });
  }

  void _showChatFilesPanel() {
    final sessionId = _sessionId;
    if (sessionId == null) return;
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (_) => SizedBox(
        height: MediaQuery.of(context).size.height * 0.5,
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 16, 8, 8),
              child: Row(
                children: [
                  const Expanded(
                    child: Text(
                      'Chat files',
                      style:
                          TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.attach_file),
                    tooltip: 'Attach file',
                    onPressed: () {
                      Navigator.pop(context);
                      _attachChatFile();
                    },
                  ),
                ],
              ),
            ),
            Expanded(
              child: ChatFilesPanel(
                chatFiles: _chatFiles,
                sessionId: sessionId,
                ownerDid: _ownerDid,
                onEditSharing: _editFileSharing,
              ),
            ),
          ],
        ),
      ),
    );
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
        dicomIngestJobId = submitDicomIngest(
            filePath: dicomFilePath, patientDidHash: patientHash);
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
      if (conditionImpactMap.isNotEmpty)
        'conditionImpactMap': conditionImpactMap,
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
      if (dicomIngestJobId != null)
        'dicomIngestJobId': dicomIngestJobId.toString(),
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
      await QappLauncher.ensureProtocol();
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

      await QappLauncher.showPanel(
        context,
        url: url,
        title: focusOrgan != null ? 'Anatomy: $focusOrgan' : 'Anatomy',
      );
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
          const SnackBar(
              content: Text('Could not read the selected DICOM path.')),
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
          ScaffoldMessenger.of(context)
              .showSnackBar(SnackBar(content: Text(e)));

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

    if (_pendingSelection != null && _replyToFragmentId == null) {
      await _prepareReplyFragment();
    }
    final replyFrag = _replyToFragmentId;
    final branchType = _selectedBranchTypeId;
    unawaited(_persistMessage(
      'user',
      text,
      replyToFragment: replyFrag,
      branchTypeId: branchType,
    ));
    unawaited(_maybeAutoTitle(text));
    _clearReplyTarget();
    setState(() => _selectedBranchTypeId = null);

    _promptController.clear();

    _scrollToBottom();

    final agentIndex = _messages.length - 1;

    try {
      final qpuCmd = await api.handleEngineChatCommand(text: text);
      if (qpuCmd.handled) {
        ref
            .read(qpuFeatureUnlockedProvider.notifier)
            .setUnlocked(qpuCmd.featureUnlocked);
        setState(() {
          _messages[agentIndex] =
              _Message(role: 'agent', content: qpuCmd.response);
          _isInferring = false;
        });
        unawaited(_persistMessage('agent', qpuCmd.response));
        _scrollToBottom();
        return;
      }

      var lifecycleJson = await catalog.getModelLifecycleStatus();
      var lifecycle = _parseLifecycleState(lifecycleJson);
      if (lifecycle != 'Active') {
        final recovered = await _ensureActiveModel();
        if (!recovered) {
          setState(() {
            _messages[agentIndex] = _Message(
              role: 'agent',
              content:
                  'No active model — open Chat environment (tune icon) to pick an installed model, or LLM Hub to download one.',
            );
            _isInferring = false;
          });
          _scrollToBottom();
          return;
        }
        lifecycleJson = await catalog.getModelLifecycleStatus();
        lifecycle = _parseLifecycleState(lifecycleJson);
      }

      if (_sessionId != null) {
        await chat.compileSessionEnvironment(sessionId: _sessionId!);
      }

      _streamSub?.cancel();
      final stream = api.runInferenceStream(
        prompt: text,
        modelPath: widget.modelPath,
        sessionId: _sessionId ?? '',
        replyToFragmentId: replyFrag,
      );

      _streamSub = stream.listen(
        (line) => _handleInferenceEvent(line, agentIndex),
        onError: (e) {
          setState(() {
            _messages[agentIndex] = _Message(
              role: 'agent',
              content: 'Error: $e',
              committed: false,
            );
            _isInferring = false;
          });
        },
        onDone: () {
          if (mounted && _isInferring) {
            setState(() => _isInferring = false);
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

  void _handleInferenceEvent(String line, int agentIndex) {
    try {
      final map = jsonDecode(line) as Map<String, dynamic>;
      final event = map['event'] as String? ?? '';
      final data = map['data'];

      if (event == 'token' && data is String) {
        setState(() {
          final prev = _messages[agentIndex];
          _messages[agentIndex] = prev.copyWith(content: prev.content + data);
        });
        _scrollToBottom();
        return;
      }

      if (event == 'error') {
        final msg = data is String ? data : data.toString();
        setState(() {
          _messages[agentIndex] = _Message(
            role: 'agent',
            content: msg,
            committed: false,
          );
          _isInferring = false;
        });
        return;
      }

      if (event == 'done' && data is Map<String, dynamic>) {
        final text = data['text'] as String? ?? _messages[agentIndex].content;
        final committed = data['committed'] as bool? ?? false;
        final blockReason = data['block_reason'] as String?;
        final prov =
            (data['provenance_hashes'] as List<dynamic>? ?? const []).length;
        final citations = <ChatCitation>[];
        for (final c in data['citations'] as List<dynamic>? ?? const []) {
          if (c is Map<String, dynamic>) {
            citations.add(ChatCitation(
              ontologyId: c['ontology_id'] as String? ?? '',
              label: c['label'] as String? ?? '',
            ));
          }
        }

        final semanticQuin = _parseSemanticQuin(data['semantic_quin']);
        final walCommitted = data['wal_committed'] as bool? ?? false;
        final walSuspended = data['wal_suspended'] as bool? ?? false;
        final suspendedAgreementId = data['suspended_agreement_id'] as int?;
        final sieveTokenCount = data['sieve_token_count'] as int? ?? 0;
        final shieldAlert = data['shield_alert'] as bool? ?? false;
        final axiomBoundsLabel = data['axiom_bounds_label'] as String?;
        final isShield = shieldAlert ||
            (blockReason != null &&
                blockReason.toLowerCase().contains('shield'));

        final display = committed
            ? text
            : (isShield)
                ? ''
                : (blockReason != null && blockReason.isNotEmpty)
                    ? '**Webizen blocked:** $blockReason\n\n$text'
                    : text;

        setState(() {
          _messages[agentIndex] = _Message(
            role: 'agent',
            content: display,
            citations: citations,
            provenanceCount: prov,
            committed: committed,
            blockReason: blockReason,
            semanticQuin: semanticQuin,
            walCommitted: walCommitted,
            walSuspended: walSuspended,
            suspendedAgreementId: suspendedAgreementId,
            sieveTokenCount: sieveTokenCount,
            shieldAlert: isShield,
            axiomBoundsLabel: axiomBoundsLabel,
            shieldMessage:
                isShield ? (blockReason ?? 'Shield intervention') : null,
          );
          _isInferring = false;
        });

        if (walSuspended) {
          ref.read(pendingAffirmationsProvider.notifier).poll();
        }

        if (committed) {
          unawaited(_persistMessage('agent', text));
          if (_ttsEnabled) {
            ChatSpeechService.instance.speakAgentResponse(text);
          }
        }
        _scrollToBottom();
      }
    } catch (_) {
      // Legacy plain-text fallback
      setState(() {
        final prev = _messages[agentIndex];
        _messages[agentIndex] = prev.copyWith(content: prev.content + line);
      });
      _scrollToBottom();
    }
  }

  Future<void> _maybeAutoTitle(String firstMessage) async {
    if (_sessionId == null) return;
    if (_sessionTitle != 'Chat' && _sessionTitle != 'New chat') return;
    final title = firstMessage.length > 48
        ? '${firstMessage.substring(0, 45)}…'
        : firstMessage;
    try {
      await chat.renameChatSession(sessionId: _sessionId!, title: title);
      if (mounted) setState(() => _sessionTitle = title);
    } catch (_) {}
  }

  void _stopInference() {
    api.cancelInferenceStream();
    setState(() => _isInferring = false);
  }

  List<int>? _parseSemanticQuin(dynamic raw) {
    if (raw is! List) return null;
    final out = <int>[];
    for (final v in raw) {
      if (v is int) {
        out.add(v);
      } else if (v is num) {
        out.add(v.toInt());
      }
    }
    return out.length >= 3 ? out : null;
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
    ref.listen<List<api.SuspendedTxView>>(pendingAffirmationsProvider, (_, __) {
      unawaited(_refreshGuardianStatuses());
    });

    final cs = Theme.of(context).colorScheme;

    if (_sessionLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    return Scaffold(
      key: _scaffoldKey,
      drawer: ChatHistoryDrawer(
        activeSessionId: _sessionId,
        onSessionSelected: _switchSession,
        onNewChat: _startNewChat,
        onGroupCreated: _switchSession,
      ),
      body: Column(
        children: [
          Material(
            color: cs.surfaceContainerLow,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
              child: Row(
                children: [
                  IconButton(
                    icon: const Icon(Icons.history),
                    tooltip: 'Chat history',
                    onPressed: () => _scaffoldKey.currentState?.openDrawer(),
                  ),
                  Expanded(
                    child: Text(
                      _sessionTitle,
                      style: Theme.of(context).textTheme.titleMedium,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.share_outlined),
                    tooltip: 'Shared ontologies (session DID)',
                    onPressed: _sessionId == null ? null : _showSessionShares,
                  ),
                  IconButton(
                    icon: Badge(
                      isLabelVisible: _chatFiles.isNotEmpty,
                      label: Text('${_chatFiles.length}'),
                      child: const Icon(Icons.folder_open_outlined),
                    ),
                    tooltip: 'Chat files',
                    onPressed: _sessionId == null ? null : _showChatFilesPanel,
                  ),
                  IconButton(
                    icon: const Icon(Icons.account_tree_outlined),
                    tooltip: 'Chat graph',
                    onPressed: _sessionId == null
                        ? null
                        : () => showModalBottomSheet(
                              context: context,
                              isScrollControlled: true,
                              builder: (_) => SizedBox(
                                height:
                                    MediaQuery.of(context).size.height * 0.55,
                                child: ChatGraphPanel(sessionId: _sessionId!),
                              ),
                            ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.tune),
                    tooltip: 'Chat environment',
                    onPressed: _sessionId == null || _isInferring
                        ? null
                        : _openEnvironmentSheet,
                  ),
                  if (_isGroup)
                    IconButton(
                      icon: const Icon(Icons.smart_toy_outlined),
                      tooltip: 'Sub-agent outcome sharing',
                      onPressed: _isInferring || _sessionId == null
                          ? null
                          : _showAgentOutcomeSharing,
                    ),
                  IconButton(
                    icon: const Icon(Icons.group_add_outlined),
                    tooltip: _isGroup ? 'Add friend' : 'Start group chat',
                    onPressed: _isInferring || _sessionId == null
                        ? null
                        : () => _openAddFriends(createGroup: !_isGroup),
                  ),
                  IconButton(
                    icon: const Icon(Icons.add_comment_outlined),
                    tooltip: 'New chat',
                    onPressed: _isInferring ? null : _startNewChat,
                  ),
                ],
              ),
            ),
          ),
          if (_participants.isNotEmpty)
            Material(
              color: cs.surfaceContainerHighest.withValues(alpha: 0.4),
              child: SizedBox(
                height: 40,
                child: ListView.separated(
                  scrollDirection: Axis.horizontal,
                  padding:
                      const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                  itemCount: _participants.length,
                  separatorBuilder: (_, __) => const SizedBox(width: 6),
                  itemBuilder: (context, i) {
                    final p = _participants[i];
                    return Chip(
                      label: Text(p.displayName,
                          style: const TextStyle(fontSize: 12)),
                      visualDensity: VisualDensity.compact,
                      avatar: p.role == 'owner'
                          ? const Icon(Icons.star, size: 14)
                          : const Icon(Icons.person_outline, size: 14),
                    );
                  },
                ),
              ),
            ),
          ChatEnvironmentBar(
            sessionId: _sessionId,
            onTap: _sessionId == null || _isInferring
                ? null
                : _openEnvironmentSheet,
          ),
          const VaultHudBar(dense: true),
          if (widget.modelPath.isEmpty)
            MaterialBanner(
              content: const Text(
                'No active model — open Chat environment to pick an installed model, or LLM Hub to download.',
              ),
              leading: const Icon(Icons.memory_outlined),
              actions: [
                TextButton(
                  onPressed: _sessionId == null ? null : _openEnvironmentSheet,
                  child: const Text('Choose model'),
                ),
                TextButton(
                  onPressed: () =>
                      ref.read(shellNavIndexProvider.notifier).state = 7,
                  child: const Text('LLM Hub'),
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

                final display = m.content.isEmpty && _isInferring && !isUser
                    ? '…'
                    : m.content;
                final authorLabel = m.authorName;
                final replyTag = m.replyToFragment;
                final semanticContextField = !isUser &&
                        m.semanticQuin != null &&
                        m.semanticQuin!.length > 3
                    ? m.semanticQuin![3]
                    : null;
                final sensitivityStyle =
                    resolveSensitivityStyle(context, semanticContextField);
                final bubbleColor = isUser
                    ? cs.primary.withValues(alpha: 0.18)
                    : semanticContextField != null
                        ? sensitivityStyle.background
                        : cs.surfaceContainerHighest;

                return Align(
                  alignment:
                      isUser ? Alignment.centerRight : Alignment.centerLeft,
                  child: Container(
                    margin: const EdgeInsets.symmetric(vertical: 4),
                    padding: const EdgeInsets.symmetric(
                        horizontal: 14, vertical: 10),
                    constraints: BoxConstraints(
                        maxWidth: MediaQuery.of(context).size.width * 0.75),
                    decoration: BoxDecoration(
                      color: bubbleColor,
                      borderRadius: BorderRadius.circular(12),
                      border: semanticContextField != null
                          ? Border.all(color: sensitivityStyle.border)
                          : null,
                    ),
                    child: Column(
                      crossAxisAlignment: isUser
                          ? CrossAxisAlignment.end
                          : CrossAxisAlignment.start,
                      children: [
                        if (authorLabel != null && authorLabel.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 4),
                            child: Text(
                              authorLabel,
                              style: Theme.of(context)
                                  .textTheme
                                  .labelSmall
                                  ?.copyWith(
                                    color: cs.secondary,
                                    fontWeight: FontWeight.w600,
                                  ),
                            ),
                          ),
                        if (!isUser && m.subAgentOf != null)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 4),
                            child: Text(
                              m.modelId != null
                                  ? 'Sub-agent · ${m.modelId}'
                                  : 'Sub-agent of participant',
                              style: Theme.of(context)
                                  .textTheme
                                  .labelSmall
                                  ?.copyWith(
                                    color: cs.tertiary,
                                    fontStyle: FontStyle.italic,
                                  ),
                            ),
                          ),
                        if (replyTag != null)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 4),
                            child: Chip(
                              label: Text(
                                  '↳ fragment ${replyTag.length > 8 ? replyTag.substring(0, 8) : replyTag}…',
                                  style: const TextStyle(fontSize: 11)),
                              visualDensity: VisualDensity.compact,
                              padding: EdgeInsets.zero,
                            ),
                          ),
                        if (!isUser && semanticContextField != null)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 6),
                            child: SensitivityBadge(
                              contextField: semanticContextField,
                              dense: true,
                            ),
                          ),
                        if (_sessionId != null && m.lamport != null)
                          ChatReactionBar(
                            sessionId: _sessionId!,
                            messageLamport: m.lamport!,
                            reactions: _reactions[m.lamport] ?? const [],
                            onChanged: _loadReactions,
                          ),
                        if (_sessionId != null)
                          Builder(builder: (context) {
                            final attached = _fileForLamport(m.lamport);
                            if (attached == null ||
                                attached.mediaKind != 'image') {
                              return const SizedBox.shrink();
                            }
                            return ChatImageAttachment(
                              sessionId: _sessionId!,
                              file: attached,
                            );
                          }),
                        isUser
                            ? (display.contains(r'$')
                                ? MarkdownMessage(
                                    content: display,
                                    style: TextStyle(color: cs.onSurface),
                                  )
                                : SelectableText(
                                    display,
                                    onSelectionChanged: (sel, _) =>
                                        _onTextSelected(
                                            i,
                                            m.lamport ?? BigInt.zero,
                                            display,
                                            sel),
                                  ))
                            : (m.semanticQuin != null && display.trim().isEmpty)
                                ? const SizedBox.shrink()
                                : SelectableText(
                                    display,
                                    onSelectionChanged: (sel, _) =>
                                        _onTextSelected(
                                            i,
                                            m.lamport ?? BigInt.zero,
                                            display,
                                            sel),
                                  ),
                        if (!isUser && m.shieldAlert)
                          ShieldAlert(
                            message: m.shieldMessage ??
                                m.blockReason ??
                                'Shield intervention',
                            boundsLabel: m.axiomBoundsLabel,
                          ),
                        if (!isUser && (m.walSuspended || m.guardianRatified))
                          GuardianAffirmationChip(
                            walSuspended: m.walSuspended,
                            ratified: m.guardianRatified,
                            agreementId: m.suspendedAgreementId,
                          ),
                        if (!isUser && m.semanticQuin != null)
                          SuperQuinProvenanceChip(
                            fields: m.semanticQuin!,
                            walCommitted: m.walCommitted,
                            sieveTokenCount: m.sieveTokenCount,
                            principalLabel: _ownerDid,
                          ),
                        if (!isUser &&
                            (m.semanticQuin == null || m.citations.isNotEmpty))
                          ChatCitationChips(
                            citations: m.citations,
                            provenanceCount: m.provenanceCount,
                            committed: m.committed,
                          ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),
          if (_isInferring) const LinearProgressIndicator(),
          if (_replyAnchorPreview != null || _pendingSelection != null)
            Material(
              color: cs.primaryContainer.withValues(alpha: 0.35),
              child: Padding(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                child: Row(
                  children: [
                    const Icon(Icons.reply, size: 18),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'Reply to: ${_replyAnchorPreview ?? _pendingSelection?.text ?? ''}',
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ),
                    if (_pendingSelection != null && _replyToFragmentId == null)
                      TextButton(
                          onPressed: _prepareReplyFragment,
                          child: const Text('Pin')),
                    if (_branchTypes.isNotEmpty)
                      PopupMenuButton<String>(
                        tooltip: 'Branch type',
                        initialValue: _selectedBranchTypeId,
                        onSelected: (v) =>
                            setState(() => _selectedBranchTypeId = v),
                        itemBuilder: (_) => _branchTypes
                            .map(
                              (t) => PopupMenuItem(
                                value: t.id,
                                child: Text('${t.emoji} ${t.label}'),
                              ),
                            )
                            .toList(),
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 8),
                          child: Text(
                            _branchTypeLabel(_selectedBranchTypeId),
                            style: Theme.of(context).textTheme.labelSmall,
                          ),
                        ),
                      ),
                    IconButton(
                      icon: const Icon(Icons.close, size: 18),
                      onPressed: _clearReplyTarget,
                    ),
                  ],
                ),
              ),
            ),
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
                  icon: Icon(_showMathKeyboard
                      ? Icons.functions
                      : Icons.calculate_outlined),
                  color: _showMathKeyboard ? cs.primary : cs.secondary,
                  tooltip: 'Math / LaTeX keyboard',
                  onPressed: _isInferring
                      ? null
                      : () => setState(
                          () => _showMathKeyboard = !_showMathKeyboard),
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
                  icon: const Icon(Icons.attach_file),
                  tooltip: 'Attach file or image',
                  onPressed: _isInferring ? null : _attachChatFile,
                ),
                IconButton(
                  icon: const Icon(Icons.biotech_outlined),
                  tooltip: 'Open latest reply in Anatomy',
                  onPressed: _isInferring ? null : () => _openLatestInAnatomy(),
                ),
                IconButton(
                  icon: const Icon(Icons.medical_services_outlined),
                  tooltip: 'Open in Anatomy with DICOM overlay',
                  onPressed:
                      _isInferring ? null : _openLatestInAnatomyWithDicom,
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
                if (_isInferring)
                  IconButton(
                    icon: const Icon(Icons.stop_circle_outlined),
                    color: cs.error,
                    tooltip: 'Stop generation',
                    onPressed: _stopInference,
                  ),
                IconButton(
                  icon: _isInferring
                      ? const SizedBox(
                          width: 20,
                          height: 20,
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
      ),
    );
  }
}

class _PendingSelection {
  final int messageIndex;
  final BigInt lamport;
  final String content;
  final int start;
  final int end;
  final String text;

  const _PendingSelection({
    required this.messageIndex,
    required this.lamport,
    required this.content,
    required this.start,
    required this.end,
    required this.text,
  });
}

class _Message {
  final String role;
  final String content;
  final BigInt? lamport;
  final String? authorName;
  final String? replyToFragment;
  final String? subAgentOf;
  final String? modelId;
  final List<ChatCitation> citations;
  final int provenanceCount;
  final bool committed;
  final String? blockReason;
  final List<int>? semanticQuin;
  final bool walCommitted;
  final bool walSuspended;
  final int? suspendedAgreementId;
  final bool guardianRatified;
  final int sieveTokenCount;
  final bool shieldAlert;
  final String? axiomBoundsLabel;
  final String? shieldMessage;

  const _Message({
    required this.role,
    required this.content,
    this.lamport,
    this.authorName,
    this.replyToFragment,
    this.subAgentOf,
    this.modelId,
    this.citations = const [],
    this.provenanceCount = 0,
    this.committed = false,
    this.blockReason,
    this.semanticQuin,
    this.walCommitted = false,
    this.walSuspended = false,
    this.suspendedAgreementId,
    this.guardianRatified = false,
    this.sieveTokenCount = 0,
    this.shieldAlert = false,
    this.axiomBoundsLabel,
    this.shieldMessage,
  });

  _Message copyWith({
    String? content,
    List<ChatCitation>? citations,
    int? provenanceCount,
    bool? committed,
    String? blockReason,
    List<int>? semanticQuin,
    bool? walCommitted,
    bool? walSuspended,
    int? suspendedAgreementId,
    bool? guardianRatified,
    int? sieveTokenCount,
    bool? shieldAlert,
    String? axiomBoundsLabel,
    String? shieldMessage,
  }) {
    return _Message(
      role: role,
      content: content ?? this.content,
      citations: citations ?? this.citations,
      provenanceCount: provenanceCount ?? this.provenanceCount,
      committed: committed ?? this.committed,
      blockReason: blockReason ?? this.blockReason,
      semanticQuin: semanticQuin ?? this.semanticQuin,
      walCommitted: walCommitted ?? this.walCommitted,
      walSuspended: walSuspended ?? this.walSuspended,
      suspendedAgreementId: suspendedAgreementId ?? this.suspendedAgreementId,
      guardianRatified: guardianRatified ?? this.guardianRatified,
      sieveTokenCount: sieveTokenCount ?? this.sieveTokenCount,
      shieldAlert: shieldAlert ?? this.shieldAlert,
      axiomBoundsLabel: axiomBoundsLabel ?? this.axiomBoundsLabel,
      shieldMessage: shieldMessage ?? this.shieldMessage,
    );
  }
}
