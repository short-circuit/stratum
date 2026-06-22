
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/search_provider.dart';
import '../providers/vault_provider.dart';
import '../widgets/note_card.dart';


class SearchScreen extends StatefulWidget {
  const SearchScreen({super.key});

  @override
  State<SearchScreen> createState() => _SearchScreenState();
}

class _SearchScreenState extends State<SearchScreen> {
  final _searchCtrl = TextEditingController();
  final _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _focusNode.requestFocus());
  }

  @override
  void dispose() {
    _searchCtrl.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final search = context.watch<SearchProvider>();
    final vault = context.watch<VaultProvider>();
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Column(children: [
      // Search bar
      Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(border: Border(bottom: BorderSide(color: Theme.of(context).dividerColor))),
        child: Row(children: [
          Expanded(
            child: TextField(
              controller: _searchCtrl,
              focusNode: _focusNode,
              onChanged: (q) => search.search(q, mode: search.mode),
              decoration: InputDecoration(
                hintText: 'Search notes...',
                prefixIcon: const Icon(Icons.search, size: 20),
                suffixIcon: search.searching
                    ? const SizedBox(width: 20, height: 20, child: Padding(padding: EdgeInsets.all(12), child: CircularProgressIndicator(strokeWidth: 2)))
                    : _searchCtrl.text.isNotEmpty
                        ? IconButton(icon: const Icon(Icons.clear, size: 18), onPressed: () { _searchCtrl.clear(); search.clear(); })
                        : null,
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                contentPadding: const EdgeInsets.symmetric(vertical: 10),
                filled: true,
                fillColor: isDark ? Colors.grey[800] : Colors.grey[100],
              ),
            ),
          ),
        ]),
      ),
      // Mode chips
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(children: [
          _modeChip('Full-text', 'fulltext', search),
          const SizedBox(width: 6),
          _modeChip('Graph', 'graph', search),
          const SizedBox(width: 6),
          _modeChip('Regex', 'regex', search),
          const Spacer(),
          if (search.hasResults) Text('${search.results.length} results', style: TextStyle(fontSize: 12, color: Colors.grey[500])),
        ]),
      ),
      // Results
      Expanded(
        child: search.hasResults
            ? ListView.builder(
                itemCount: search.results.length,
                itemBuilder: (ctx, i) {
                  final r = search.results[i];
                  return NoteCard(
                    path: r.path,
                    title: r.title,
                    snippet: r.snippet,
                    onTap: () => vault.openNote(r.path),
                  );
                },
              )
            : _searchCtrl.text.isNotEmpty
                ? Center(child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.search_off, size: 48, color: Colors.grey[400]),
                      const SizedBox(height: 12),
                      Text('No results found', style: TextStyle(fontSize: 16, color: Colors.grey[500])),
                    ],
                  ))
                : Center(child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.search, size: 64, color: Colors.grey[300]),
                      const SizedBox(height: 16),
                      Text('Search your notes', style: TextStyle(fontSize: 18, color: Colors.grey[500])),
                      const SizedBox(height: 8),
                      Text('Full-text, graph-aware, or regex search', style: TextStyle(fontSize: 13, color: Colors.grey[400])),
                    ],
                  )),
      ),
    ]);
  }

  Widget _modeChip(String label, String value, SearchProvider search) {
    final selected = search.mode == value;
    return GestureDetector(
      onTap: () => search.setMode(value),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        decoration: BoxDecoration(
          color: selected ? Theme.of(context).colorScheme.primary.withValues(alpha: 0.2) : null,
          borderRadius: BorderRadius.circular(12),
          border: selected ? Border.all(color: Theme.of(context).colorScheme.primary) : null,
        ),
        child: Text(label, style: TextStyle(fontSize: 12, color: selected ? Theme.of(context).colorScheme.primary : Colors.grey)),
      ),
    );
  }
}
