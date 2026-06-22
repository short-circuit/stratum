import 'package:flutter/material.dart';
import '../models/note.dart';

enum BlockType { heading, paragraph, codeBlock, listItem, quote, rule, empty }

class MarkdownBlock {
  final BlockType type;
  String raw;
  final int headingLevel;
  final String? language;

  MarkdownBlock({
    required this.type,
    required this.raw,
    this.headingLevel = 0,
    this.language,
  });
}

ParsedMarkdown parseMarkdown(String body) {
  final lines = body.split('\n');
  final blocks = <MarkdownBlock>[];

  String frontmatterRaw = '';
  Frontmatter frontmatter = Frontmatter();
  int i = 0;

  if (lines.isNotEmpty && lines[0].trim() == '---') {
    final fmLines = <String>[];
    i = 1;
    while (i < lines.length && lines[i].trim() != '---') {
      fmLines.add(lines[i]);
      i++;
    }
    if (i < lines.length) i++;
    frontmatterRaw = fmLines.join('\n');
    frontmatter = _parseFrontmatter(fmLines);
  }

  while (i < lines.length) {
    final line = lines[i];

    if (line.trim().isEmpty) {
      i++;
      continue;
    }

    if (line.trimLeft().startsWith('```')) {
      final lang = line.trimLeft().substring(3).trim();
      final codeLines = <String>[];
      i++;
      while (i < lines.length && !lines[i].trimLeft().startsWith('```')) {
        codeLines.add(lines[i]);
        i++;
      }
      if (i < lines.length) i++;
      blocks.add(MarkdownBlock(
        type: BlockType.codeBlock,
        raw: '```$lang\n${codeLines.join('\n')}\n```',
        language: lang.isNotEmpty ? lang : null,
      ));
      continue;
    }

    final headingMatch = RegExp(r'^(#{1,6})\s+(.+)$').firstMatch(line);
    if (headingMatch != null) {
      final level = headingMatch.group(1)!.length;
      blocks.add(MarkdownBlock(type: BlockType.heading, raw: line, headingLevel: level));
      i++;
      continue;
    }

    if (RegExp(r'^[*-_]{3,}\s*$').hasMatch(line.trim())) {
      blocks.add(MarkdownBlock(type: BlockType.rule, raw: line));
      i++;
      continue;
    }

    if (line.startsWith('> ')) {
      final quoteLines = <String>[];
      while (i < lines.length && lines[i].startsWith('> ')) {
        quoteLines.add(lines[i]);
        i++;

      }
      blocks.add(MarkdownBlock(type: BlockType.quote, raw: quoteLines.join('\n')));
      continue;
    }

    if (RegExp(r'^\s*[-*+]\s+|^\s*\d+\.\s+').hasMatch(line)) {
      blocks.add(MarkdownBlock(type: BlockType.listItem, raw: line));
      i++;
      continue;
    }

    final paraLines = <String>[];
    while (i < lines.length &&
        lines[i].trim().isNotEmpty &&
        !lines[i].trimLeft().startsWith('```') &&
        !RegExp(r'^(#{1,6})\s+').hasMatch(lines[i]) &&
        !RegExp(r'^[*-_]{3,}\s*$').hasMatch(lines[i].trim()) &&
        !lines[i].startsWith('> ') &&
        !RegExp(r'^\s*[-*+]\s+|^\s*\d+\.\s+').hasMatch(lines[i])) {
      paraLines.add(lines[i]);
      i++;
    }
    blocks.add(MarkdownBlock(type: BlockType.paragraph, raw: paraLines.join('\n')));
  }

  return ParsedMarkdown(frontmatterRaw: frontmatterRaw, frontmatter: frontmatter, blocks: blocks);
}

Frontmatter _parseFrontmatter(List<String> lines) {
  String? title;
  String? created;
  String? modified;
  final tags = <String>[];
  final aliases = <String>[];

  for (final line in lines) {
    final kv = RegExp(r'^(\w[\w_-]*):\s*(.*)$').firstMatch(line);
    if (kv == null) continue;
    final key = kv.group(1)!;
    final value = kv.group(2)!.trim();
    switch (key) {
      case 'title': title = value; break;
      case 'created': created = value; break;
      case 'modified': modified = value; break;
      case 'tags': tags.addAll(_parseListValue(value)); break;
      case 'aliases': aliases.addAll(_parseListValue(value)); break;
    }
  }

  return Frontmatter(title: title, created: created, modified: modified, tags: tags, aliases: aliases);
}

List<String> _parseListValue(String value) {
  value = value.trim();
  if (value.startsWith('[') && value.endsWith(']')) {
    final inner = value.substring(1, value.length - 1);
    return inner.split(',').map((s) => s.trim().replaceAll('"', '').replaceAll("'", '')).where((s) => s.isNotEmpty).toList();
  }
  return value.split(',').map((s) => s.trim()).where((s) => s.isNotEmpty).toList();
}

void _extractInlineLinks(String text, List<Link> links, int lineNum) {
  final wikiRe = RegExp(r'\[\[([^\]|]+)(?:\|([^\]]+))?\]\]');
  for (final m in wikiRe.allMatches(text)) {
    links.add(Link(target: m.group(1)!.trim(), displayText: m.group(2)?.trim(), line: lineNum));
  }
}

void _extractInlineTags(String text, List<Tag> tags) {
  final tagRe = RegExp(r'(?:^|\s)#([a-zA-Z][\w-]*)');
  for (final m in tagRe.allMatches(text)) {
    tags.add(Tag(name: m.group(1)!, source: 'inline'));
  }
}

List<Link> extractLinks(String body) {
  final links = <Link>[];
  final lines = body.split('\n');
  for (int i = 0; i < lines.length; i++) {
    _extractInlineLinks(lines[i], links, i + 1);
  }
  return links;
}

List<Tag> extractTags(String body) {
  final tags = <Tag>[];
  final lines = body.split('\n');
  for (final line in lines) {
    _extractInlineTags(line, tags);
  }
  return tags;
}

/// Render a block's raw markdown text as a RichText with inline formatting.
Widget buildBlockWidget(MarkdownBlock block, ThemeData theme, bool isDark) {
  final textColor = isDark ? Colors.grey[300]! : Colors.grey[800]!;

  switch (block.type) {
    case BlockType.heading:
      final scale = [1.6, 1.4, 1.25, 1.1, 1.0, 0.9][block.headingLevel - 1].clamp(0.9, 1.6);
      final text = block.raw.replaceFirst(RegExp(r'^#+\s*'), '');
      return Padding(
        padding: EdgeInsets.only(top: block.headingLevel == 1 ? 16 : 12, bottom: 4),
        child: RichText(
          text: _parseInline(text, TextStyle(
            fontSize: 17 * scale,
            fontWeight: FontWeight.w700,
            color: isDark ? Colors.grey[200]! : Colors.grey[900]!,
            height: 1.3,
          )),
        ),
      );

    case BlockType.quote:
      final text = block.raw.replaceAll(RegExp(r'^> ', multiLine: true), '');
      return Container(
        margin: const EdgeInsets.only(bottom: 4),
        padding: const EdgeInsets.only(left: 12, top: 4, bottom: 4),
        decoration: BoxDecoration(
          border: Border(left: BorderSide(color: isDark ? Colors.grey[600]! : Colors.grey[400]!, width: 3)),
        ),
        child: RichText(
          text: _parseInline(text, TextStyle(
            fontSize: 14,
            color: textColor,
            fontStyle: FontStyle.italic,
            height: 1.6,
          )),
        ),
      );

    case BlockType.codeBlock:
      final lang = block.language;
      final code = _codeContent(block.raw);
      return Container(
        width: double.infinity,
        margin: const EdgeInsets.only(bottom: 8),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: isDark ? const Color(0xFF1E1E1E) : Colors.grey[200]!,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            if (lang != null && lang.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text(lang, style: TextStyle(fontSize: 11, color: Colors.grey[500], fontWeight: FontWeight.w600)),
              ),
            Text(code, style: TextStyle(fontFamily: 'monospace', fontSize: 13, color: textColor, height: 1.5)),
          ],
        ),
      );

    case BlockType.listItem:
      final isCheckbox = RegExp(r'^\s*[-*+]\s+\[([ x])\]\s+').firstMatch(block.raw);
      if (isCheckbox != null) {
        final checked = isCheckbox.group(1) == 'x';
        final text = block.raw.replaceFirst(RegExp(r'^\s*[-*+]\s+\[[ x]\]\s+'), '');
        return Padding(
          padding: const EdgeInsets.only(bottom: 2),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.only(top: 1, right: 6),
                child: Icon(
                  checked ? Icons.check_box : Icons.check_box_outline_blank,
                  size: 18,
                  color: checked
                      ? (isDark ? Colors.green[400] : Colors.green[600])
                      : Colors.grey[500],
                ),
              ),
              Expanded(
                child: RichText(
                  text: TextSpan(
                    style: TextStyle(fontSize: 14, color: checked ? Colors.grey[500]! : textColor, height: 1.6,
                      decoration: checked ? TextDecoration.lineThrough : null),
                    children: _parseInlineSpans(text, TextStyle(fontSize: 14, color: checked ? Colors.grey[500]! : textColor, height: 1.6,
                      decoration: checked ? TextDecoration.lineThrough : null)),
                  ),
                ),
              ),
            ],
          ),
        );
      }
      final text = block.raw.replaceFirst(RegExp(r'^\s*[-*+]\s+|\s*\d+\.\s+'), '');
      return Padding(
        padding: const EdgeInsets.only(bottom: 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.only(top: 2, right: 6),
              child: Text('•', style: TextStyle(fontSize: 14, color: textColor)),
            ),
            Expanded(
              child: RichText(
                text: _parseInline(text, TextStyle(fontSize: 14, color: textColor, height: 1.6)),
              ),
            ),
          ],
        ),
      );

    case BlockType.rule:
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Divider(color: Colors.grey[400], thickness: 1),
      );

    case BlockType.paragraph:
      return Padding(
        padding: const EdgeInsets.only(bottom: 4),
        child: RichText(
          text: _parseInline(block.raw, TextStyle(fontSize: 14, color: textColor, height: 1.6)),
        ),
      );

    case BlockType.empty:
      return const SizedBox(height: 8);
  }
}

TextSpan _parseInline(String text, TextStyle baseStyle) {
  return TextSpan(children: _parseInlineSpans(text, baseStyle));
}

List<InlineSpan> _parseInlineSpans(String text, TextStyle baseStyle) {
  final spans = <InlineSpan>[];
  final regex = RegExp(
    r'(\*\*|__)(.+?)\1'                // bold
    r'|(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)' // italic *
    r'|(?<!_)_(?!_)(.+?)(?<!_)_(?!_)'  // italic _
    r'|~~(.+?)~~'                       // strikethrough
    r'|`(.+?)`'                         // inline code
    r'|\[\[([^\]]+)\]\]'               // wiki link
    r'|\[([^\]]+)\]\(([^)]+)\)'        // markdown link [text](url)
    r'|#([a-zA-Z][\w-]*)'             // tag
    r'|!\[([^\]]*)\]\(([^)]+)\)'       // image
  );

  int lastEnd = 0;
  for (final match in regex.allMatches(text)) {
    if (match.start > lastEnd) {
      spans.add(TextSpan(text: text.substring(lastEnd, match.start), style: baseStyle));
    }

    TextStyle? spanStyle;
    String? displayText;
    String? tagName;

    if (match.group(1) != null) {
      // Bold **__  
      spanStyle = baseStyle.copyWith(fontWeight: FontWeight.w700);
      displayText = match.group(2);
    } else if (match.group(3) != null || match.group(4) != null) {
      // Italic
      spanStyle = baseStyle.copyWith(fontStyle: FontStyle.italic);
      displayText = match.group(3) ?? match.group(4);
    } else if (match.group(5) != null) {
      // Strikethrough
      spanStyle = baseStyle.copyWith(decoration: TextDecoration.lineThrough);
      displayText = match.group(5);
    } else if (match.group(6) != null) {
      // Inline code
      spanStyle = baseStyle.copyWith(
        fontFamily: 'monospace',
        fontSize: (baseStyle.fontSize ?? 14) * 0.92,
        backgroundColor: Colors.grey.withValues(alpha: 0.2),
      );
      displayText = match.group(6);
    } else if (match.group(7) != null) {
      // Wiki link
      spanStyle = baseStyle.copyWith(
        color: Colors.blue[400],
        decoration: TextDecoration.underline,
      );
      displayText = match.group(7);
    } else if (match.group(8) != null && match.group(9) != null) {
      // Markdown link [text](url)
      spanStyle = baseStyle.copyWith(
        color: Colors.blue[400],
        decoration: TextDecoration.underline,
      );
      displayText = match.group(8);
    } else if (match.group(10) != null) {
      // Tag
      spanStyle = baseStyle.copyWith(
        color: Colors.blue[300],
        fontWeight: FontWeight.w500,
      );
      tagName = match.group(10);
      displayText = '#$tagName';
    } else if (match.group(11) != null && match.group(12) != null) {
      // Image
      spanStyle = baseStyle.copyWith(
        color: Colors.grey[500],
        fontStyle: FontStyle.italic,
      );
      displayText = '[image: ${match.group(11)}]';
    }

    if (displayText != null) {
      spans.add(TextSpan(text: displayText, style: spanStyle));
    }

    lastEnd = match.end;
  }

  if (lastEnd < text.length) {
    spans.add(TextSpan(text: text.substring(lastEnd), style: baseStyle));
  }

  if (spans.isEmpty) {
    spans.add(TextSpan(text: text, style: baseStyle));
  }

  return spans;
}

String _codeContent(String raw) {
  final lines = raw.split('\n');
  if (lines.length <= 2) return '';
  return lines.sublist(1, lines.length - 1).join('\n');
}

class ParsedMarkdown {
  final String frontmatterRaw;
  final Frontmatter frontmatter;
  final List<MarkdownBlock> blocks;

  ParsedMarkdown({
    this.frontmatterRaw = '',
    Frontmatter? frontmatter,
    this.blocks = const [],
  }) : frontmatter = frontmatter ?? Frontmatter();
}
