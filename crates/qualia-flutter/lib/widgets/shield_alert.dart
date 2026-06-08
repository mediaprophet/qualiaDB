import 'package:flutter/material.dart';

/// Distinct chip when Webizen / Sentinel clips an anachronism or axiom violation.
class ShieldAlert extends StatelessWidget {
  final String message;
  final String? boundsLabel;

  const ShieldAlert({
    super.key,
    required this.message,
    this.boundsLabel,
  });

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Material(
        color: cs.errorContainer.withValues(alpha: 0.85),
        borderRadius: BorderRadius.circular(10),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Icons.shield_outlined, size: 20, color: cs.onErrorContainer),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Shield',
                      style: Theme.of(context).textTheme.labelLarge?.copyWith(
                            color: cs.onErrorContainer,
                            fontWeight: FontWeight.w700,
                          ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      message,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: cs.onErrorContainer,
                          ),
                    ),
                    if (boundsLabel != null && boundsLabel!.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 4),
                        child: Text(
                          'Axiom window $boundsLabel',
                          style: Theme.of(context).textTheme.labelSmall?.copyWith(
                                color: cs.onErrorContainer.withValues(alpha: 0.85),
                                fontFamily: 'monospace',
                              ),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
