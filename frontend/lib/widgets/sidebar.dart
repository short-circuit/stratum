import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/vault_provider.dart';
import 'note_card.dart';
import 'tag_chip.dart';

class Sidebar extends StatefulWidget {
  final ValueChanged<String> onNoteSelected;
  final VoidCallback onCreateNote;
  final VoidCallback onOpenSettings;

  const Sidebar({super.key, required this.onNoteSelected, required this.onCreateNote, required this.onOpenSettings});

  @override
  State<Sidebar> createState() => _SidebarState();
}

class _SidebarState extends State<Sidebar> with SingleTickerProviderStateMixin {
  late TabController _tabCtrl;
  final _searchCtrl = TextEditingController();
  final Set<String> _expandedFolders = {};

  @override
  void initState() {
    super.initState();
    _tabCtrl = TabController(length: 2, vsync: this);
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  @override
  void dispose() {
    _tabCtrl.dispose();
    _searchCtrl.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final vault = context.read<VaultProvider>();
    await Future.wait([vault.loadNotes(), vault.loadTagCloud(), vault.loadStats()]);
  }

  @override
  Widget build(BuildContext context) {
    final vault = context.watch<VaultProvider>();
    final isDark = Theme.of(context).brightness == Brightness.dark;
    const sidebarWidth = 260.0;

    return SizedBox(
      width: sidebarWidth,
      child: Column(
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(
              color: isDark ? Colors.grey[900] : Colors.grey[50],
              border: Border(bottom: BorderSide(color: Theme.of(context).dividerColor)),
            ),
            child: Row(children: [
              Icon(Icons.auto_stories, size: 22, color: Theme.of(context).colorScheme.primary),
              const SizedBox(width: 8),
              const Flexible(child: Text('Stratum', style: TextStyle(fontWeight: FontWeight.w700, fontSize: 16))),
              const Spacer(),
              IconButton(icon: const Icon(Icons.create_new_folder, size: 18), onPressed: widget.onCreateNote, tooltip: 'New Note', constraints: const BoxConstraints(minWidth: 32, minHeight: 32)),
              IconButton(icon: const Icon(Icons.settings, size: 18), onPressed: widget.onOpenSettings, tooltip: 'Settings', constraints: const BoxConstraints(minWidth: 32, minHeight: 32)),
            ]),
          ),
          Padding(
            padding: const EdgeInsets.all(8),
            child: TextField(
              controller: _searchCtrl,
              onChanged: (q) => vault.setSearchQuery(q),
              decoration: InputDecoration(
                hintText: 'Filter notes...',
                prefixIcon: const Icon(Icons.search, size: 16),
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
                contentPadding: const EdgeInsets.symmetric(vertical: 8),
                filled: true,
                fillColor: isDark ? Colors.grey[800] : Colors.grey[100],
                isDense: true,
              ),
            ),
          ),
          TabBar(
            controller: _tabCtrl,
            tabs: const [
              Tab(text: 'Notes'),
              Tab(text: 'Tags'),
            ],
            labelStyle: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
            indicatorSize: TabBarIndicatorSize.label,
          ),
          Expanded(
            child: TabBarView(
              controller: _tabCtrl,
              children: [
                _buildNotesList(vault, isDark),
                _buildTagsList(vault, isDark),
              ],
            ),
          ),
          if (vault.filterTag != null)
            Container(
              padding: const EdgeInsets.all(8),
              color: Theme.of(context).colorScheme.primary.withValues(alpha: 0.1),
              child: Row(children: [
                Icon(Icons.filter_alt, size: 16, color: Theme.of(context).colorScheme.primary),
                const SizedBox(width: 6),
                Text('Filtered by: ${vault.filterTag}', style: TextStyle(fontSize: 12, color: Theme.of(context).colorScheme.primary)),
                const Spacer(),
                GestureDetector(child: Icon(Icons.close, size: 16), onTap: () => vault.setFilterTag(null)),
              ]),
            ),
        ],
      ),
    );
  }

  Widget _buildNotesList(VaultProvider vault, bool isDark) {
    if (vault.loading) return const Center(child: CircularProgressIndicator());
    if (vault.error != null) return Center(child: Text(vault.error!, style: const TextStyle(color: Colors.red)));
    if (vault.notePaths.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.note_add, size: 48, color: Colors.grey[400]),
            const SizedBox(height: 12),
            Text('No notes yet', style: TextStyle(fontSize: 14, color: Colors.grey[500])),
            const SizedBox(height: 4),
            Text('Tap + to create one', style: TextStyle(fontSize: 12, color: Colors.grey[400])),
          ],
        ),
      );
    }

    // Group notes by folder
    final groups = <String, List<String>>{};
    for (final path in vault.notePaths) {
      final parts = path.split('/');
      if (parts.length > 1) {
        final folder = parts.sublist(0, parts.length - 1).join('/');
        groups.putIfAbsent(folder, () => []);
        groups[folder]!.add(path);
      } else {
        groups.putIfAbsent('', () => []);
        groups['']!.add(path);
      }
    }

    // Sort folders
    final sortedFolders = groups.keys.toList()..sort();

    return ListView.builder(
      itemCount: sortedFolders.length,
      itemBuilder: (ctx, i) {
        final folder = sortedFolders[i];
        final paths = groups[folder]!;
        final isExpanded = _expandedFolders.contains(folder) || sortedFolders.length == 1;
        final isSelected = vault.currentNote != null && paths.contains(vault.currentNote!.path);

        if (folder.isEmpty && paths.length == 1) {
          // Single root note — show without folder
          final path = paths.first;
          final title = _displayTitle(path);
          return NoteCard(
            path: path,
            title: title,
            selected: vault.currentNote?.path == path,
            onTap: () => widget.onNoteSelected(path),
          );
        }

        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            GestureDetector(
              onTap: () => setState(() {
                if (isExpanded) {
                  _expandedFolders.remove(folder);
                } else {
                  _expandedFolders.add(folder);
                }
              }),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                color: isDark ? Colors.grey[850] : Colors.grey[100],
                child: Row(
                  children: [
                    Icon(
                      isExpanded ? Icons.folder_open : Icons.folder,
                      size: 16,
                      color: isSelected ? Theme.of(context).colorScheme.primary : Colors.grey[500],
                    ),
                    const SizedBox(width: 6),
                    Text(
                      folder.isEmpty ? 'notes' : folder,
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                        color: isSelected ? Theme.of(context).colorScheme.primary : Colors.grey[600],
                      ),
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '(${paths.length})',
                      style: TextStyle(fontSize: 11, color: Colors.grey[500]),
                    ),
                    const Spacer(),
                    Icon(
                      isExpanded ? Icons.expand_less : Icons.expand_more,
                      size: 16,
                      color: Colors.grey[500],
                    ),
                  ],
                ),
              ),
            ),
            if (isExpanded)
              ...paths.map((path) {
                final title = path.split('/').last.replaceAll('.md', '').replaceAll('-', ' ');
                return NoteCard(
                  path: path,
                  title: title,
                  selected: vault.currentNote?.path == path,
                  onTap: () => widget.onNoteSelected(path),
                  indent: true,
                );
              }),
          ],
        );
      },
    );
  }

  String _displayTitle(String path) {
    return path.split('/').last.replaceAll('.md', '').replaceAll('-', ' ');
  }

  Widget _buildTagsList(VaultProvider vault, bool isDark) {
    if (vault.tagCloud.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.tag, size: 48, color: Colors.grey[400]),
            const SizedBox(height: 12),
            Text('No tags yet', style: TextStyle(fontSize: 14, color: Colors.grey[500])),
          ],
        ),
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.all(8),
      itemCount: vault.tagCloud.length,
      itemBuilder: (ctx, i) {
        final t = vault.tagCloud[i];
        final selected = vault.filterTag == t.name;
        return Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: TagChip(
            label: t.name,
            count: t.count,
            selected: selected,
            onTap: () => vault.setFilterTag(selected ? null : t.name),
          ),
        );
      },
    );
  }
}
