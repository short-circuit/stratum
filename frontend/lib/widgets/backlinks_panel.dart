import 'package:flutter/material.dart';
import '../models/note.dart';

class BacklinksPanel extends StatelessWidget {
  final List<Backlink> backlinks;
  final List<String> unlinkedMentions;
  final ValueChanged<String> onNavigate;

  const BacklinksPanel({super.key, required this.backlinks, this.unlinkedMentions = const [], required this.onNavigate});

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, mainAxisSize: MainAxisSize.min, children: [
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        child: Text('Backlinks', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 12, color: cs.onSurface.withValues(alpha: 0.6))),
      ),
      if (backlinks.isEmpty && unlinkedMentions.isEmpty)
        const Padding(
          padding: EdgeInsets.symmetric(horizontal: 12),
          child: Text('No backlinks', style: TextStyle(fontSize: 12, color: Colors.grey)),
        )
      else
        Expanded(
          child: ListView(
            padding: EdgeInsets.zero,
            children: [
              ...backlinks.map((b) => ListTile(
                dense: true,
                visualDensity: VisualDensity.compact,
                leading: Icon(Icons.link, size: 14, color: Colors.blue[400]),
                title: Text(b.sourceTitle, style: const TextStyle(fontSize: 12)),
                subtitle: b.contextSnippet.isNotEmpty
                    ? Text(b.contextSnippet, maxLines: 1, overflow: TextOverflow.ellipsis, style: TextStyle(fontSize: 11, color: Colors.grey))
                    : null,
                onTap: () => onNavigate(b.sourcePath),
              )),
              if (unlinkedMentions.isNotEmpty) ...[
                Padding(
                  padding: const EdgeInsets.fromLTRB(12, 4, 12, 2),
                  child: Text('Unlinked mentions', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 11, color: Colors.grey[500])),
                ),
                ...unlinkedMentions.map((m) => Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 1),
                  child: Text(m, style: TextStyle(fontSize: 11, color: Colors.orange[400])),
                )),
              ],
            ],
          ),
        ),
    ]);
  }
}
