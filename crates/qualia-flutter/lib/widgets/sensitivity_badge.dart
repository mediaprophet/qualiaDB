import 'package:flutter/material.dart';

enum SensitivityLevel { public, restricted, classified }

SensitivityLevel sensitivityLevelFromByte(int? level) {
  return switch (level ?? 0) {
    0x01 => SensitivityLevel.restricted,
    0x02 => SensitivityLevel.classified,
    _ => SensitivityLevel.public,
  };
}

SensitivityLevel sensitivityLevelFromContextField(int? contextField) {
  if (contextField == null) return SensitivityLevel.public;
  return sensitivityLevelFromByte((contextField >> 56) & 0xFF);
}

String sensitivityLabelFromContextField(int? contextField) {
  return sensitivityLabelFromLevel((contextField ?? 0) >> 56);
}

String sensitivityLabelFromLevel(int? level) {
  return switch (sensitivityLevelFromByte(level)) {
    SensitivityLevel.restricted => 'RESTRICTED',
    SensitivityLevel.classified => 'CLASSIFIED',
    SensitivityLevel.public => 'PUBLIC',
  };
}

class SensitivityStyle {
  final String label;
  final IconData icon;
  final Color foreground;
  final Color background;
  final Color border;
  final bool sanctuaryMode;

  const SensitivityStyle({
    required this.label,
    required this.icon,
    required this.foreground,
    required this.background,
    required this.border,
    this.sanctuaryMode = false,
  });
}

SensitivityStyle resolveSensitivityStyle(
  BuildContext context,
  int? contextField,
) {
  return resolveSensitivityStyleFromLevel(
    context,
    contextField == null ? null : ((contextField >> 56) & 0xFF),
  );
}

SensitivityStyle resolveSensitivityStyleFromLevel(
  BuildContext context,
  int? level,
) {
  final cs = Theme.of(context).colorScheme;
  return switch (sensitivityLevelFromByte(level)) {
    SensitivityLevel.restricted => SensitivityStyle(
        label: 'RESTRICTED',
        icon: Icons.shield_outlined,
        foreground: const Color(0xFFFFD166),
        background: const Color(0xFFFFD166).withValues(alpha: 0.14),
        border: const Color(0xFFFFD166).withValues(alpha: 0.45),
      ),
    SensitivityLevel.classified => SensitivityStyle(
        label: 'CLASSIFIED',
        icon: Icons.lock_outline,
        foreground: const Color(0xFF9ADCF2),
        background: Colors.black.withValues(alpha: 0.72),
        border: const Color(0xFF9ADCF2).withValues(alpha: 0.45),
        sanctuaryMode: true,
      ),
    SensitivityLevel.public => SensitivityStyle(
        label: 'PUBLIC',
        icon: Icons.public_outlined,
        foreground: cs.primary,
        background: cs.primary.withValues(alpha: 0.10),
        border: cs.primary.withValues(alpha: 0.25),
      ),
  };
}

class SensitivityBadge extends StatelessWidget {
  final int? contextField;
  final int? sensitivityLevel;
  final bool dense;

  const SensitivityBadge({
    super.key,
    this.contextField,
    this.sensitivityLevel,
    this.dense = false,
  });

  @override
  Widget build(BuildContext context) {
    final style = sensitivityLevel != null
        ? resolveSensitivityStyleFromLevel(context, sensitivityLevel)
        : resolveSensitivityStyle(context, contextField);
    final iconSize = dense ? 14.0 : 16.0;
    final fontSize = dense ? 10.0 : 11.0;
    final padding = dense
        ? const EdgeInsets.symmetric(horizontal: 8, vertical: 4)
        : const EdgeInsets.symmetric(horizontal: 10, vertical: 6);

    return Container(
      padding: padding,
      decoration: BoxDecoration(
        color: style.background,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: style.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(style.icon, size: iconSize, color: style.foreground),
          const SizedBox(width: 6),
          Text(
            style.sanctuaryMode ? '${style.label} - Sanctuary' : style.label,
            style: TextStyle(
              color: style.foreground,
              fontSize: fontSize,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.3,
            ),
          ),
        ],
      ),
    );
  }
}
