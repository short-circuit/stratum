
import 'package:flutter/material.dart';
import '../models/note.dart';
import 'tag_chip.dart';

class NoteCard extends StatelessWidget {
  final String path;
  final String title;
  final String? snippet;
  final List<String>? tags;
  final bool selected;
  final bool indent;
  final VoidCallback onTap;

  const NoteCard({super.key, required this.path, required this.title, this.snippet, this.tags, this.selected = false, this.indent = false, required this.onTap});

  factory NoteCard.fromNote(Note note, {bool selected = false, required VoidCallback onTap}) => NoteCard(
    path: note.path, title: note.title, snippet: note.body.split('\n').firstWhere((l) => !l.startsWith('#') && l.trim().isNotEmpty, orElse: () => ''), tags: note.tags.map((t) => t.name).toSet().toList(), selected: selected, onTap: onTap,
  );

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: selected ? Theme.of(context).colorScheme.primary.withValues(alpha: 0.1) : null,
        border: Border(bottom: BorderSide(color: Theme.of(context).dividerColor, width: 0.5)),
      ),
      child: ListTile(
        dense: true,
        contentPadding: EdgeInsets.only(left: indent ? 32 : 16, right: 8),
        title: Text(title, style: TextStyle(fontWeight: selected ? FontWeight.w600 : FontWeight.w400, fontSize: 14)),
        subtitle: snippet != null && snippet!.isNotEmpty ? Text(snippet!, maxLines: 1, overflow: TextOverflow.ellipsis, style: TextStyle(fontSize: 12, color: Colors.grey)) : null,
        trailing: tags != null && tags!.isNotEmpty ? SizedBox(
          width: 120, child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(children: tags!.take(3).map((t) => Padding(padding: const EdgeInsets.only(left: 2), child: TagChip(label: t))).toList())),
        ) : null,
        onTap: onTap,
      ),
    );
  }
}
