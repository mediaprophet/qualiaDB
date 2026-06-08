import 'package:flutter/material.dart';

/// Temporal + spatial axiom bounds for chat inference (LTL / Allen binding).
class AxiomBoundsSheet extends StatefulWidget {
  final int startYear;
  final int endYear;
  final String spatialContext;

  const AxiomBoundsSheet({
    super.key,
    required this.startYear,
    required this.endYear,
    this.spatialContext = '',
  });

  @override
  State<AxiomBoundsSheet> createState() => _AxiomBoundsSheetState();
}

class _AxiomBoundsSheetState extends State<AxiomBoundsSheet> {
  late int _start;
  late int _end;
  late TextEditingController _spatial;

  @override
  void initState() {
    super.initState();
    _start = widget.startYear;
    _end = widget.endYear;
    _spatial = TextEditingController(text: widget.spatialContext);
  }

  @override
  void dispose() {
    _spatial.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Axiom bounds', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(
              'Temporal and spatial limits validated before inference prefill.',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: cs.onSurfaceVariant,
                  ),
            ),
            const SizedBox(height: 16),
            Text('Year window: $_start – $_end', style: Theme.of(context).textTheme.labelMedium),
            RangeSlider(
              values: RangeValues(_start.toDouble(), _end.toDouble()),
              min: 1000,
              max: 2100,
              divisions: 110,
              labels: RangeLabels('$_start', '$_end'),
              onChanged: (v) => setState(() {
                _start = v.start.round();
                _end = v.end.round();
              }),
            ),
            TextField(
              controller: _spatial,
              decoration: const InputDecoration(
                labelText: 'Spatial context (optional)',
                hintText: 'e.g. Western Front 1944',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.pop(context),
                  child: const Text('Cancel'),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: () => Navigator.pop(context, {
                    'start_year': _start,
                    'end_year': _end,
                    'spatial_context': _spatial.text.trim(),
                  }),
                  child: const Text('Apply bounds'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
