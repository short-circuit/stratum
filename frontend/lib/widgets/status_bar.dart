
import 'package:flutter/material.dart';

class StatusBar extends StatelessWidget {
  final String syncStatus;
  final int backlinkCount;
  final int wordCount;
  final bool syncing;

  const StatusBar({super.key, required this.syncStatus, this.backlinkCount = 0, this.wordCount = 0, this.syncing = false});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        color: isDark ? Colors.grey[900] : Colors.grey[100],
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: Row(children: [
        if (syncing)
          SizedBox(width: 12, height: 12, child: CircularProgressIndicator(strokeWidth: 1.5))
        else
          Icon(Icons.cloud_done, size: 14, color: Colors.green[400]),
        const SizedBox(width: 6),
        Text(syncStatus, style: TextStyle(fontSize: 11, color: Colors.grey[500])),
        const Spacer(),
        if (backlinkCount > 0) ...[
          Icon(Icons.link, size: 12, color: Colors.grey[500]),
          const SizedBox(width: 4),
          Text('$backlinkCount backlinks', style: TextStyle(fontSize: 11, color: Colors.grey[500])),
          const SizedBox(width: 16),
        ],
        if (wordCount > 0) Text('$wordCount words', style: TextStyle(fontSize: 11, color: Colors.grey[500])),
      ]),
    );
  }
}
