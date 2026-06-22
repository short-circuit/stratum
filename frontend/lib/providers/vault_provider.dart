import 'package:flutter/foundation.dart';
import '../models/note.dart';
import '../models/graph.dart';
import '../services/rust_backend.dart';
import '../services/mock_backend.dart';
import '../services/markdown_parser.dart';

class VaultProvider extends ChangeNotifier {
  final MockBackend _backend = MockBackend();
  String _vaultPath = '';

  String get vaultPath => _vaultPath;

  List<String> _notePaths = [];
  Note? _currentNote;
  GraphLayout? _graph;
  List<TagCount> _tagCloud = [];
  VaultStats? _stats;
  bool _loading = false;
  String? _error;
  String _searchQuery = '';
  String? _filterTag;

  List<String> get notePaths => _notePaths;
  Note? get currentNote => _currentNote;
  GraphLayout? get graph => _graph;
  List<TagCount> get tagCloud => _tagCloud;
  VaultStats? get stats => _stats;
  bool get loading => _loading;
  String? get error => _error;
  String get searchQuery => _searchQuery;
  String? get filterTag => _filterTag;

  List<TagCount> get currentNoteTags {
    if (_currentNote == null) return [];
    final counts = <String, int>{};
    for (final t in _currentNote!.tags) { counts[t.name] = (counts[t.name] ?? 0) + 1; }
    return counts.entries.map((e) => TagCount(name: e.key, count: e.value)).toList();
  }

  void _setLoading(bool v) { _loading = v; notifyListeners(); }
  void _setError(String? e) { _error = e; notifyListeners(); }

  Future<void> configureVault(String path) async {
    if (path == _vaultPath) return;
    _vaultPath = path;
    _backend.setVaultPath(path);
    await Future.wait([loadNotes(), loadGraph(), loadTagCloud(), loadStats()]);
  }

  Future<void> loadNotes() async {
    _setLoading(true);
    try {
      _notePaths = await _backend.listNotes(tag: _filterTag, search: _searchQuery.isNotEmpty ? _searchQuery : null);
      _setError(null);
    } catch (e) { _setError('Failed to load notes: $e'); }
    _setLoading(false);
  }

  Future<void> openNote(String path) async {
    _setLoading(true);
    try {
      _currentNote = await _backend.openNote(path);
      _setError(null);
    } catch (e) { _setError('Failed to open note: $e'); }
    _setLoading(false);
  }

  Future<void> saveCurrentNote() async {
    if (_currentNote == null) return;
    try {
      await _backend.saveNote(_currentNote!.path, _currentNote!.frontmatter, _currentNote!.body);
      await loadNotes();
      await loadGraph();
      await loadTagCloud();
      await loadStats();
    } catch (e) { _setError('Failed to save: $e'); }
  }

  Future<void> createNote(String path, {String? title}) async {
    _setLoading(true);
    try {
      _currentNote = await _backend.createNote(path, title: title);
      await loadNotes();
      await loadGraph();
      await loadTagCloud();
      await loadStats();
      _setError(null);
    } catch (e) { _setError('Failed to create note: $e'); }
    _setLoading(false);
  }

  Future<void> deleteNote(String path) async {
    try {
      await _backend.deleteNote(path);
      if (_currentNote?.path == path) _currentNote = null;
      await loadNotes();
      await loadGraph();
      await loadTagCloud();
      await loadStats();
    } catch (e) { _setError('Failed to delete: $e'); }
  }

  Future<void> loadGraph() async {
    try {
      _graph = await _backend.getGraph();
      notifyListeners();
    } catch (e) { _setError('Failed to load graph: $e'); }
  }

  Future<void> loadTagCloud() async {
    try { _tagCloud = await _backend.getTagCloud(); notifyListeners(); }
    catch (e) { _setError('Failed to load tags: $e'); }
  }

  Future<void> loadStats() async {
    try { _stats = await _backend.getStats(); notifyListeners(); }
    catch (e) { _setError('Failed to load stats: $e'); }
  }

  void setSearchQuery(String q) { _searchQuery = q; loadNotes(); }
  void setFilterTag(String? tag) { _filterTag = tag; loadNotes(); }

  void updateCurrentBody(String body) {
    if (_currentNote == null) return;
    final links = extractLinks(body);
    final tags = extractTags(body);
    _currentNote = Note(
      path: _currentNote!.path,
      title: _currentNote!.title,
      frontmatter: _currentNote!.frontmatter,
      body: body,
      links: links,
      tags: [..._currentNote!.frontmatter.tags.map((t) => Tag(name: t, source: 'frontmatter')), ...tags],
      backlinks: _currentNote!.backlinks,
      unlinkedMentions: _currentNote!.unlinkedMentions,
      modifiedAt: DateTime.now(),
    );
    notifyListeners();
  }

  void updateCurrentFrontmatter(Frontmatter fm) {
    if (_currentNote == null) return;
    final inlineTags = _currentNote!.tags.where((t) => t.source == 'inline').toList();
    _currentNote = Note(
      path: _currentNote!.path,
      title: fm.title ?? _currentNote!.title,
      frontmatter: fm,
      body: _currentNote!.body,
      links: _currentNote!.links,
      tags: [...fm.tags.map((t) => Tag(name: t, source: 'frontmatter')), ...inlineTags],
      backlinks: _currentNote!.backlinks,
      unlinkedMentions: _currentNote!.unlinkedMentions,
      modifiedAt: DateTime.now(),
    );
    notifyListeners();
  }
}
