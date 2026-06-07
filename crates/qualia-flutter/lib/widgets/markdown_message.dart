import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_math_fork/flutter_math.dart';

/// Renders chat messages with Markdown, inline `$...$`, and `$$...$$` display math.
class MarkdownMessage extends StatelessWidget {
  final String content;
  final TextStyle? style;

  const MarkdownMessage({super.key, required this.content, this.style});

  @override
  Widget build(BuildContext context) {
    final baseStyle = style ?? const TextStyle(fontSize: 14);
    if (!content.contains(r'$')) {
      return MarkdownBody(
        data: content,
        selectable: true,
        styleSheet: MarkdownStyleSheet(p: baseStyle),
      );
    }

    final widgets = <Widget>[];
    var cursor = 0;
    final re = RegExp(r'\$\$(.+?)\$\$|(?<!\$)\$(?!\$)(.+?)(?<!\$)\$(?!\$)', dotAll: true);
    for (final m in re.allMatches(content)) {
      if (m.start > cursor) {
        widgets.add(_mdSegment(content.substring(cursor, m.start), baseStyle));
      }
      final tex = (m.group(1) ?? m.group(2) ?? '').trim();
      final display = m.group(1) != null;
      widgets.add(Padding(
        padding: EdgeInsets.symmetric(vertical: display ? 6 : 2),
        child: Math.tex(
          tex,
          textStyle: baseStyle.copyWith(fontSize: display ? 16 : 14),
          mathStyle: display ? MathStyle.display : MathStyle.text,
        ),
      ));
      cursor = m.end;
    }
    if (cursor < content.length) {
      widgets.add(_mdSegment(content.substring(cursor), baseStyle));
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
  }

  Widget _mdSegment(String data, TextStyle baseStyle) {
    if (!data.contains(r'$')) {
      return MarkdownBody(
        data: data,
        selectable: true,
        styleSheet: MarkdownStyleSheet(p: baseStyle),
      );
    }
    return MarkdownMessage(content: data, style: baseStyle);
  }
}
