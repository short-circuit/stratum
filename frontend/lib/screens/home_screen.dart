
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/vault_provider.dart';
import '../widgets/sidebar.dart';
import 'editor_screen.dart';
import 'graph_screen.dart';
import 'search_screen.dart';
import 'settings_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _currentIndex = 0;
  final List<Widget> _screens = [
    const EditorScreen(),
    const GraphScreen(),
    const SearchScreen(),
  ];

  void _onNoteSelected(String path) {
    context.read<VaultProvider>().openNote(path);
    setState(() => _currentIndex = 0);
  }

  @override
  Widget build(BuildContext context) {
    final vault = context.watch<VaultProvider>();
    return Scaffold(
      body: Column(
        children: [
          Expanded(
            child: Row(
              children: [
                // Sidebar
                Sidebar(
                  onNoteSelected: _onNoteSelected,
                  onCreateNote: () => _showCreateNoteDialog(context),
                  onOpenSettings: () => Navigator.push(context, MaterialPageRoute(builder: (_) => const SettingsScreen())),
                ),
                // Vertical divider
                Container(width: 1, color: Theme.of(context).dividerColor),
                // Main content area
                Expanded(
                  flex: 3,
                  child: _screens[_currentIndex],
                ),
              ],
            ),
          ),
          // Bottom dock
          _buildBottomDock(context, vault),
        ],
      ),
    );
  }

  Widget _buildBottomDock(BuildContext context, VaultProvider vault) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Container(
      height: 44,
      decoration: BoxDecoration(
        color: isDark ? Colors.grey[900] : Colors.grey[100],
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: Row(
        children: [
          _dockItem(Icons.edit_note, 'Editor', 0),
          _dockItem(Icons.hub, 'Graph', 1),
          _dockItem(Icons.search, 'Search', 2),
          const Spacer(),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Text('${vault.stats?.noteCount ?? 0} notes', style: TextStyle(fontSize: 12, color: Colors.grey[500])),
          ),
        ],
      ),
    );
  }

  Widget _dockItem(IconData icon, String label, int index) {
    final selected = _currentIndex == index;
    return GestureDetector(
      onTap: () => setState(() => _currentIndex = index),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        decoration: BoxDecoration(
          border: Border(top: BorderSide(color: selected ? Theme.of(context).colorScheme.primary : Colors.transparent, width: 2)),
        ),
        child: Row(
          children: [
            Icon(icon, size: 18, color: selected ? Theme.of(context).colorScheme.primary : Colors.grey),
            const SizedBox(width: 6),
            Text(label, style: TextStyle(fontSize: 13, color: selected ? Theme.of(context).colorScheme.primary : Colors.grey)),
          ],
        ),
      ),
    );
  }

  void _showCreateNoteDialog(BuildContext context) {
    final controller = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Create Note'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(
            hintText: 'Note title',
            helperText: 'Creates <title>.md in vault root',
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(onPressed: () async {
            if (controller.text.trim().isEmpty) return;
            final slug = controller.text.trim().toLowerCase().replaceAll(RegExp(r'\s+'), '-');
            await context.read<VaultProvider>().createNote('$slug.md', title: controller.text.trim());
            if (ctx.mounted) Navigator.pop(ctx);
          }, child: const Text('Create')),
        ],
      ),
    );
  }
}
