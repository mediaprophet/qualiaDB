import 'dart:convert';
import 'dart:ui';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../screens/llm_hub_screen.dart';
import '../screens/ontology_hub_screen.dart';
import '../screens/qualia_qapp_webview.dart';
import '../src/rust/api/qapp_api.dart';
import '../src/rust/api/qualia_api.dart';
import '../src/rust/api/qualia_api_extras.dart' as api_extras;

class QappVaultScreen extends StatefulWidget {
  const QappVaultScreen({super.key});

  @override
  State<QappVaultScreen> createState() => _QappVaultScreenState();
}

class _QappVaultScreenState extends State<QappVaultScreen> {
  final List<Map<String, String>> _qapps = [];
  final Map<String, Map<String, dynamic>> _readinessReports = {};
  String _vcInput = '';
  String _generatedVc = '';
  String _launchingId = '';
  String _installStatus = '';

  @override
  void initState() {
    super.initState();
    _loadInstalledQapps(notifyUpdates: true);
  }

  Future<void> _loadInstalledQapps({bool notifyUpdates = false}) async {
    try {
      final names = await listInstalledQapps();
      final qappEntries = <Map<String, String>>[];
      final reports = <String, Map<String, dynamic>>{};
      final pendingUpdates = <String>[];

      for (final name in names) {
        String status = 'Installed';
        String summary = 'No readiness report available yet.';
        String version = 'unknown';
        String updateAvailable = 'false';
        String offeredVersion = '';
        String updateMessage = '';

        try {
          version = installedQappVersion(qappName: name) ?? 'unknown';
        } catch (e) {
          version = 'unknown';
        }

        try {
          final updateJson = checkQappUpdate(qappName: name);
          final update = jsonDecode(updateJson) as Map<String, dynamic>;
          if (update['update_available'] == true) {
            updateAvailable = 'true';
            offeredVersion =
                (update['offered_version'] as String?) ?? offeredVersion;
            updateMessage = (update['message'] as String?) ?? '';
            pendingUpdates.add(name);
          }
        } catch (e) {
          debugPrint('Qapp update check skipped for $name: $e');
        }

        try {
          final reportJson =
              await api_extras.inspectInstalledQappReadiness(qappName: name);
          final report = jsonDecode(reportJson) as Map<String, dynamic>;
          reports[name] = report;
          final ready = report['ready'] == true;
          final blockingIssues = (report['blocking_issues'] ?? 0) as num;
          status = ready ? 'Ready' : '$blockingIssues missing';
          summary = (report['summary'] as String?) ?? summary;
        } catch (e) {
          status = 'Readiness unknown';
          summary = 'Could not evaluate qapp requirements: $e';
          reports[name] = {
            'qapp_name': name,
            'ready': false,
            'summary': summary,
            'blocking_issues': 0,
            'optional_warnings': 0,
            'checks': <Map<String, dynamic>>[],
          };
        }

        qappEntries.add({
          'id': name,
          'name': name,
          'status': status,
          'summary': summary,
          'version': version,
          'updateAvailable': updateAvailable,
          'offeredVersion': offeredVersion,
          'updateMessage': updateMessage,
          'vc': 'Valid',
        });
      }

      if (mounted) {
        setState(() {
          _qapps
            ..clear()
            ..addAll(qappEntries);
          _readinessReports
            ..clear()
            ..addAll(reports);
        });

        if (notifyUpdates && pendingUpdates.isNotEmpty) {
          final first = pendingUpdates.first;
          final entry = qappEntries.firstWhere((q) => q['id'] == first);
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                entry['updateMessage']?.isNotEmpty == true
                    ? entry['updateMessage']!
                    : 'Update available for $first',
              ),
              action: SnackBarAction(
                label: 'Update',
                onPressed: () => _applyQappUpdate(first),
              ),
              duration: const Duration(seconds: 12),
            ),
          );
        }
      }
    } catch (e) {
      if (mounted) setState(() => _installStatus = 'Failed to load qapps: $e');
    }
  }

  Future<void> _applyQappUpdate(String qappName) async {
    final entry = _qapps.firstWhere(
      (q) => q['id'] == qappName,
      orElse: () => {'offeredVersion': '', 'version': 'unknown'},
    );
    final current = entry['version'] ?? 'unknown';
    final offered = entry['offeredVersion'] ?? '';

    final proceed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Update $qappName?'),
        content: Text(
          offered.isNotEmpty
              ? 'Replace v$current with v$offered from the bundled package.'
              : 'Replace the installed copy with the bundled package.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Update'),
          ),
        ],
      ),
    );
    if (proceed != true || !mounted) return;

    setState(() => _installStatus = 'Updating $qappName...');
    try {
      final result = applyQappUpdate(qappName: qappName);
      if (mounted) {
        setState(() => _installStatus = result);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(result)),
        );
        await _loadInstalledQapps();
      }
    } catch (e) {
      if (mounted) {
        setState(() => _installStatus = 'Update failed: $e');
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Update failed: $e')),
        );
      }
    }
  }

  Future<void> _handleLaunch(String qappName) async {
    setState(() => _launchingId = qappName);
    try {
      try {
        registerQappFromInstalledManifest(qappName: qappName);
      } catch (_) {
        // Manifest may already be registered; launch still proceeds.
      }
      final url = await launchInstalledQapp(qappName: qappName);
      if (!mounted) return;
      if (url.startsWith('http://127.0.0.1') || url.startsWith('http://localhost')) {
        await Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => QualiaQappWebView(url: url, title: qappName),
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
          SnackBar(content: Text('Launch failed: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _launchingId = '');
    }
  }

  Future<void> _handleInstallQapp() async {
    final dir = await FilePicker.platform.getDirectoryPath(
      dialogTitle: 'Select qapp directory (must contain qapp.json)',
    );
    if (dir == null) return;

    final qappName = dir.split(RegExp(r'[/\\]')).last;

    try {
      final updateJson = checkQappUpdateFromPath(
        qappName: qappName,
        sourcePath: dir,
      );
      final update = jsonDecode(updateJson) as Map<String, dynamic>;
      final offered = update['offered_version'] as String? ?? '';
      final installed = update['installed_version'] as String?;
      final hasUpdate = update['update_available'] == true;

      if (hasUpdate && mounted) {
        final proceed = await showDialog<bool>(
          context: context,
          builder: (ctx) => AlertDialog(
            title: Text('Upgrade $qappName?'),
            content: Text(
              installed == null
                  ? 'Install v$offered from the selected package.'
                  : 'Replace v$installed with v$offered from the selected package.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(ctx, false),
                child: const Text('Cancel'),
              ),
              ElevatedButton(
                onPressed: () => Navigator.pop(ctx, true),
                child: Text(installed == null ? 'Install' : 'Upgrade'),
              ),
            ],
          ),
        );
        if (proceed != true || !mounted) return;

        setState(() => _installStatus = 'Installing...');
        final result = applyQappUpdateFromPath(
          qappName: qappName,
          sourcePath: dir,
        );
        if (mounted) {
          setState(() => _installStatus = result);
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(result)),
          );
          await _loadInstalledQapps();
        }
        return;
      }

      if (!hasUpdate && installed != null && mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              (update['message'] as String?) ??
                  '$qappName v$installed is already up to date.',
            ),
          ),
        );
        return;
      }
    } catch (e) {
      debugPrint('Install path version check skipped: $e');
    }

    setState(() => _installStatus = 'Installing...');
    try {
      final credential = await generateQappCredential(qappName: qappName);
      final result = await verifyAndInstallQapp(
        zipPath: dir,
        credentialSig: credential,
      );
      if (mounted) {
        setState(() => _installStatus = result);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Installed: $qappName')),
        );
        await _loadInstalledQapps();
      }
    } catch (e) {
      if (mounted) {
        setState(() => _installStatus = 'Error: $e');
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Install failed: $e')),
        );
      }
    }
  }

  Future<void> _handleSignVc() async {
    if (_vcInput.isEmpty) return;
    final vc = await generateQappCredential(qappName: _vcInput);
    setState(() => _generatedVc = vc);
  }

  Future<void> _showProbeResult(String endpointOrId) async {
    try {
      final probeJson = await api_extras.testSparqlEndpoint(
        endpointOrId: endpointOrId,
      );
      if (!mounted) return;
      final probe = jsonDecode(probeJson) as Map<String, dynamic>;
      final reachable = probe['reachable'] == true;
      final statusCode = probe['status_code']?.toString() ?? 'n/a';
      final detail = probe['detail']?.toString() ?? 'No detail provided.';
      final resolved = probe['resolved_endpoint']?.toString() ?? endpointOrId;
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text('SPARQL Probe: $endpointOrId'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Resolved endpoint: $resolved'),
              const SizedBox(height: 8),
              Text('Reachable: ${reachable ? "yes" : "no"}'),
              Text('HTTP status: $statusCode'),
              const SizedBox(height: 12),
              Text(detail),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Probe failed: $e')),
        );
      }
    }
  }

  IconData _statusIcon(String status) {
    switch (status) {
      case 'ready':
        return Icons.check_circle_outline;
      case 'missing':
        return Icons.error_outline;
      case 'inactive':
        return Icons.pause_circle_outline;
      case 'cataloged':
      case 'declared':
        return Icons.info_outline;
      default:
        return Icons.help_outline;
    }
  }

  Color _statusColor(String status) {
    switch (status) {
      case 'ready':
        return const Color(0xFF00FF88);
      case 'missing':
        return const Color(0xFFFF6B6B);
      case 'inactive':
        return const Color(0xFFFFD166);
      default:
        return const Color(0xFF9ADCF2);
    }
  }

  Future<void> _showReadinessInspector(String qappName) async {
    final report = _readinessReports[qappName];
    if (report == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No readiness report loaded yet.')),
      );
      return;
    }

    final checks = ((report['checks'] as List?) ?? const [])
        .map((item) => Map<String, dynamic>.from(item as Map))
        .toList();

    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (context) => DraggableScrollableSheet(
        expand: false,
        initialChildSize: 0.78,
        builder: (context, scrollController) => Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Text(
                      '$qappName readiness',
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                  ),
                  IconButton(
                    tooltip: 'Refresh report',
                    onPressed: () async {
                      Navigator.of(context).pop();
                      await _loadInstalledQapps();
                      if (mounted) {
                        await _showReadinessInspector(qappName);
                      }
                    },
                    icon: const Icon(Icons.refresh),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(report['summary']?.toString() ?? 'No summary available.'),
              const SizedBox(height: 16),
              Text(
                'Blocking issues: ${report['blocking_issues'] ?? 0} | Optional warnings: ${report['optional_warnings'] ?? 0}',
                style: const TextStyle(fontSize: 12, color: Colors.grey),
              ),
              const SizedBox(height: 20),
              Expanded(
                child: ListView.separated(
                  controller: scrollController,
                  itemCount: checks.length,
                  separatorBuilder: (_, __) => const Divider(height: 16),
                  itemBuilder: (context, index) {
                    final check = checks[index];
                    final kind = check['kind']?.toString() ?? 'unknown';
                    final id = check['id']?.toString() ?? 'unknown';
                    final status = check['status']?.toString() ?? 'unknown';
                    final detail = check['detail']?.toString() ?? '';
                    final required = check['required'] == true;

                    Widget? action;
                    if (kind == 'ontology') {
                      action = TextButton.icon(
                        onPressed: () {
                          Navigator.of(context).pop();
                          Navigator.of(this.context).push(
                            MaterialPageRoute(
                              builder: (_) => const OntologyHubScreen(),
                            ),
                          );
                        },
                        icon: const Icon(Icons.account_tree_outlined),
                        label: Text(required ? 'Manage ontology' : 'Open ontology hub'),
                      );
                    } else if (kind == 'model') {
                      action = TextButton.icon(
                        onPressed: () {
                          Navigator.of(context).pop();
                          Navigator.of(this.context).push(
                            MaterialPageRoute(
                              builder: (_) => const LLMHubScreen(),
                            ),
                          );
                        },
                        icon: const Icon(Icons.memory_outlined),
                        label: Text(required ? 'Manage model' : 'Open model hub'),
                      );
                    } else if (kind == 'sparql-endpoint') {
                      action = TextButton.icon(
                        onPressed: () => _showProbeResult(id),
                        icon: const Icon(Icons.network_check),
                        label: const Text('Probe endpoint'),
                      );
                    }

                    return Container(
                      padding: const EdgeInsets.all(14),
                      decoration: BoxDecoration(
                        color: Colors.black.withOpacity(0.24),
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(color: Colors.white.withOpacity(0.06)),
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Icon(
                                _statusIcon(status),
                                color: _statusColor(status),
                                size: 18,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  '$kind | $id',
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                              ),
                              Text(
                                status,
                                style: TextStyle(
                                  color: _statusColor(status),
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 8),
                          Text(
                            detail,
                            style: const TextStyle(fontSize: 12, color: Colors.grey),
                          ),
                          if (action != null) ...[
                            const SizedBox(height: 10),
                            action,
                          ],
                        ],
                      ),
                    );
                  },
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
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _buildGlassContainer(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Row(
                        children: [
                          const Icon(
                            Icons.apps,
                            color: Color(0xFF00F0FF),
                            size: 28,
                          ),
                          const SizedBox(width: 12),
                          const Text(
                            'Qapp Vault',
                            style: TextStyle(
                              color: Colors.white,
                              fontSize: 20,
                              fontWeight: FontWeight.bold,
                              fontFamily: 'monospace',
                            ),
                          ),
                        ],
                      ),
                      ElevatedButton.icon(
                        onPressed: _handleInstallQapp,
                        icon: const Icon(Icons.add_box, size: 16),
                        label: const Text('Install Package'),
                        style: ElevatedButton.styleFrom(
                          backgroundColor:
                              const Color(0xFF00F0FF).withOpacity(0.1),
                          foregroundColor: const Color(0xFF00F0FF),
                          side: BorderSide(
                            color: const Color(0xFF00F0FF).withOpacity(0.3),
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Install and manage third-party edge-native qapps. Qapps are sandboxed and verified via VCs.',
                    style: TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                  const SizedBox(height: 24),
                  if (_qapps.isEmpty)
                    const Center(
                      child: Padding(
                        padding: EdgeInsets.symmetric(vertical: 32),
                        child: Text(
                          'No qapps installed — place qapp directories in your Qapps/ data folder.',
                          style: TextStyle(
                            color: Colors.grey,
                            fontFamily: 'monospace',
                            fontSize: 12,
                          ),
                        ),
                      ),
                    )
                  else
                    Column(
                      children: _qapps.map((qapp) {
                        final isLaunching = _launchingId == qapp['id'];
                        return Container(
                          margin: const EdgeInsets.only(bottom: 16),
                          padding: const EdgeInsets.all(16),
                          decoration: BoxDecoration(
                            color: Colors.black.withOpacity(0.4),
                            borderRadius: BorderRadius.circular(12),
                            border: Border.all(
                              color: Colors.white.withOpacity(0.05),
                            ),
                          ),
                          child: Row(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      qapp['name']!,
                                      style: const TextStyle(
                                        color: Colors.white,
                                        fontWeight: FontWeight.bold,
                                        fontSize: 16,
                                      ),
                                    ),
                                    const SizedBox(height: 8),
                                    Row(
                                      children: [
                                        const Icon(
                                          Icons.tag,
                                          color: Color(0xFF9ADCF2),
                                          size: 12,
                                        ),
                                        const SizedBox(width: 4),
                                        Text(
                                          'v${qapp['version'] ?? 'unknown'}',
                                          style: const TextStyle(
                                            color: Color(0xFF9ADCF2),
                                            fontFamily: 'monospace',
                                            fontSize: 10,
                                            fontWeight: FontWeight.w600,
                                          ),
                                        ),
                                        if (qapp['updateAvailable'] == 'true') ...[
                                          const SizedBox(width: 10),
                                          Container(
                                            padding: const EdgeInsets.symmetric(
                                              horizontal: 8,
                                              vertical: 2,
                                            ),
                                            decoration: BoxDecoration(
                                              color: const Color(0xFFFFD166)
                                                  .withOpacity(0.15),
                                              borderRadius:
                                                  BorderRadius.circular(999),
                                              border: Border.all(
                                                color: const Color(0xFFFFD166)
                                                    .withOpacity(0.35),
                                              ),
                                            ),
                                            child: Text(
                                              'v${qapp['offeredVersion'] ?? '?'} available',
                                              style: const TextStyle(
                                                color: Color(0xFFFFD166),
                                                fontFamily: 'monospace',
                                                fontSize: 10,
                                                fontWeight: FontWeight.w600,
                                              ),
                                            ),
                                          ),
                                        ],
                                        const SizedBox(width: 16),
                                        const Icon(
                                          Icons.shield,
                                          color: Color(0xFF00FF88),
                                          size: 12,
                                        ),
                                        const SizedBox(width: 4),
                                        Text(
                                          'VC: ${qapp['vc']}',
                                          style: const TextStyle(
                                            color: Color(0xFF00FF88),
                                            fontFamily: 'monospace',
                                            fontSize: 10,
                                          ),
                                        ),
                                        const SizedBox(width: 16),
                                        Text(
                                          'ID: ${qapp['id']}',
                                          style: const TextStyle(
                                            color: Colors.grey,
                                            fontFamily: 'monospace',
                                            fontSize: 10,
                                          ),
                                        ),
                                      ],
                                    ),
                                    const SizedBox(height: 8),
                                    Text(
                                      'Readiness: ${qapp['status']}',
                                      style: TextStyle(
                                        color: qapp['status'] == 'Ready'
                                            ? const Color(0xFF00FF88)
                                            : const Color(0xFFFFD166),
                                        fontFamily: 'monospace',
                                        fontSize: 11,
                                        fontWeight: FontWeight.w600,
                                      ),
                                    ),
                                    const SizedBox(height: 4),
                                    SizedBox(
                                      width: 420,
                                      child: Text(
                                        qapp['summary'] ?? '',
                                        style: const TextStyle(
                                          color: Colors.grey,
                                          fontSize: 11,
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 16),
                              Column(
                                crossAxisAlignment: CrossAxisAlignment.end,
                                children: [
                                  if (qapp['updateAvailable'] == 'true')
                                    Padding(
                                      padding: const EdgeInsets.only(bottom: 8),
                                      child: ElevatedButton.icon(
                                        onPressed: () =>
                                            _applyQappUpdate(qapp['id']!),
                                        icon: const Icon(Icons.system_update, size: 16),
                                        label: const Text('Update'),
                                        style: ElevatedButton.styleFrom(
                                          backgroundColor: const Color(0xFFFFD166)
                                              .withOpacity(0.15),
                                          foregroundColor: const Color(0xFFFFD166),
                                        ),
                                      ),
                                    ),
                                  OutlinedButton.icon(
                                    onPressed: () =>
                                        _showReadinessInspector(qapp['id']!),
                                    icon: const Icon(Icons.fact_check_outlined, size: 16),
                                    label: const Text('Inspect'),
                                  ),
                                  const SizedBox(height: 8),
                                  ElevatedButton.icon(
                                    onPressed: isLaunching
                                        ? null
                                        : () => _handleLaunch(qapp['id']!),
                                    icon: Icon(
                                      isLaunching ? Icons.sync : Icons.play_arrow,
                                      size: 16,
                                    ),
                                    label: Text(
                                      isLaunching ? 'Launching...' : 'Launch',
                                    ),
                                    style: ElevatedButton.styleFrom(
                                      backgroundColor:
                                          Colors.white.withOpacity(0.1),
                                      foregroundColor: Colors.white,
                                    ),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        );
                      }).toList(),
                    ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),
          _buildGlassContainer(
            child: Container(
              decoration: BoxDecoration(
                border: Border.all(
                  color: const Color(0xFFFFD700).withOpacity(0.3),
                ),
                borderRadius: BorderRadius.circular(16),
              ),
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Icon(
                        Icons.key,
                        color: Color(0xFFFFD700),
                        size: 28,
                      ),
                      const SizedBox(width: 12),
                      const Text(
                        'Developer Credentials',
                        style: TextStyle(
                          color: Colors.white,
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Generate Verifiable Credentials (VCs) to self-sign your own local qapps before loading them into the daemon.',
                    style: TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                  const SizedBox(height: 24),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          onChanged: (value) => _vcInput = value,
                          style: const TextStyle(
                            color: Colors.white,
                            fontFamily: 'monospace',
                            fontSize: 14,
                          ),
                          decoration: InputDecoration(
                            hintText: 'Qapp ID (e.g. com.my.qapp)',
                            hintStyle: const TextStyle(color: Colors.grey),
                            filled: true,
                            fillColor: Colors.black.withOpacity(0.5),
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(8),
                              borderSide: BorderSide.none,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 16),
                      ElevatedButton(
                        onPressed: _handleSignVc,
                        style: ElevatedButton.styleFrom(
                          backgroundColor:
                              const Color(0xFFFFD700).withOpacity(0.1),
                          foregroundColor: const Color(0xFFFFD700),
                          padding: const EdgeInsets.symmetric(
                            horizontal: 24,
                            vertical: 20,
                          ),
                        ),
                        child: const Text(
                          'Sign & Generate VC',
                          style: TextStyle(fontWeight: FontWeight.bold),
                        ),
                      ),
                    ],
                  ),
                  if (_generatedVc.isNotEmpty) ...[
                    const SizedBox(height: 16),
                    Container(
                      padding: const EdgeInsets.all(16),
                      decoration: BoxDecoration(
                        color: const Color(0xFF00FF88).withOpacity(0.05),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                          color: const Color(0xFF00FF88).withOpacity(0.2),
                        ),
                      ),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              _generatedVc,
                              style: const TextStyle(
                                color: Color(0xFF00FF88),
                                fontFamily: 'monospace',
                                fontSize: 12,
                              ),
                            ),
                          ),
                          IconButton(
                            icon: const Icon(
                              Icons.copy,
                              color: Color(0xFF00FF88),
                              size: 16,
                            ),
                            onPressed: () {
                              Clipboard.setData(
                                ClipboardData(text: _generatedVc),
                              );
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(
                                  content: Text('VC copied to clipboard'),
                                ),
                              );
                            },
                          ),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          if (_installStatus.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(
              _installStatus,
              style: const TextStyle(color: Colors.grey, fontSize: 12),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildGlassContainer({required Widget child}) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(16),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white.withOpacity(0.03),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: Colors.white.withOpacity(0.1)),
          ),
          child: child,
        ),
      ),
    );
  }
}
