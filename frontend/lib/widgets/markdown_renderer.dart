
import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';

class MarkdownRenderer extends StatelessWidget {
  final String body;
  final ValueChanged<String>? onLinkTap;
  final ScrollController? scrollController;

  const MarkdownRenderer({super.key, required this.body, this.onLinkTap, this.scrollController});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Markdown(
      data: body,
      controller: scrollController,
      selectable: true,
      styleSheet: MarkdownStyleSheet(
        h1: TextStyle(fontSize: 24, fontWeight: FontWeight.w700, color: isDark ? Colors.white : Colors.black),
        h2: TextStyle(fontSize: 20, fontWeight: FontWeight.w600, color: isDark ? Colors.white : Colors.black87),
        h3: TextStyle(fontSize: 17, fontWeight: FontWeight.w600, color: isDark ? Colors.white : Colors.black87),
        p: TextStyle(fontSize: 15, height: 1.6, color: isDark ? Colors.grey[300] : Colors.grey[800]),
        code: TextStyle(backgroundColor: isDark ? Colors.grey[800] : Colors.grey[200], fontSize: 13),
        codeblockDecoration: BoxDecoration(color: isDark ? Colors.grey[850] : Colors.grey[100], borderRadius: BorderRadius.circular(8)),
        blockquoteDecoration: BoxDecoration(border: Border(left: BorderSide(color: Colors.blue[400]!, width: 3)), color: isDark ? Colors.blue.withValues(alpha: 0.05) : Colors.blue.withValues(alpha: 0.03)),
        listBullet: TextStyle(color: isDark ? Colors.grey[400] : Colors.grey[600]),
        checkbox: TextStyle(color: Colors.green),
      ),
      onTapLink: (text, href, title) {
        if (href != null && onLinkTap != null) {
          // Handle [[wiki-links]] - they're passed as href
          onLinkTap!(href);
        }
      },
      builders: {},
    );
  }
}
