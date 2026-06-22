
import 'package:flutter/material.dart';

class TagChip extends StatelessWidget {
  final String label;
  final int? count;
  final bool selected;
  final VoidCallback? onTap;
  final VoidCallback? onRemove;

  const TagChip({super.key, required this.label, this.count, this.selected = false, this.onTap, this.onRemove});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
        decoration: BoxDecoration(
          color: selected
              ? Theme.of(context).colorScheme.primary.withValues(alpha: 0.2)
              : (isDark ? Colors.white.withValues(alpha: 0.08) : Colors.grey.withValues(alpha: 0.15)),
          borderRadius: BorderRadius.circular(12),
          border: selected ? Border.all(color: Theme.of(context).colorScheme.primary) : null,
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.tag, size: 14, color: selected ? Theme.of(context).colorScheme.primary : Colors.grey),
            const SizedBox(width: 4),
            Text(label, style: TextStyle(fontSize: 12, color: selected ? Theme.of(context).colorScheme.primary : null)),
            if (count != null) ...[
              const SizedBox(width: 4),
              Text('($count)', style: TextStyle(fontSize: 11, color: Colors.grey)),
            ],
            if (onRemove != null) ...[
              const SizedBox(width: 4),
              GestureDetector(child: Icon(Icons.close, size: 14, color: Colors.grey), onTap: onRemove),
            ],
          ],
        ),
      ),
    );
  }
}
