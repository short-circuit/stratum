import 'dart:io';
import 'dart:math';
import '../models/note.dart';
import '../models/graph.dart';
import '../models/config.dart';
import 'markdown_parser.dart';
import 'rust_backend.dart';

class MockBackend implements RustBackend {
  final _storage = <String, Note>{};
  String _vaultPath = '';

  MockBackend();

  void setVaultPath(String path) => _vaultPath = path;

  void _ensureSamples() {
    if (_storage.isNotEmpty) return;
    final d = Directory(_vaultPath);
    final hasFiles = _vaultPath.isNotEmpty && d.existsSync() && d.listSync().isNotEmpty;

    if (!hasFiles) {
      _storage.addAll(_sampleNotes());
      _rebuildBacklinks();
      if (_vaultPath.isNotEmpty) _writeAllToDisk();
    } else {
      _loadFromDisk();
    }
  }

  Map<String, Note> _sampleNotes() => {
    'welcome.md': Note(
      path: 'welcome.md', title: 'Welcome to Stratum',
      frontmatter: Frontmatter(title: 'Welcome to Stratum', tags: ['getting-started', 'welcome'], created: '2026-06-22'),
      body: '# Welcome to Stratum\n\nA privacy-first, offline-capable PKM system.\n\n## Key Features\n\n- **Plain Markdown files** — zero lock-in\n- **Bi-directional links** — `[[Wiki Links]]`\n- **Full-text search** — instant, offline\n- **Git sync** — automatic backups\n- **Graph view** — visualize connections\n\n## Getting Started\n\nCreate your first note with [[Note About Life]] or browse the [[Example Note]] to see what\'s possible.\n\n#welcome #pkm',
      links: [Link(target: 'Note About Life', line: 12), Link(target: 'Example Note', line: 12)],
      tags: [Tag(name: 'getting-started', source: 'frontmatter'), Tag(name: 'welcome', source: 'frontmatter'), Tag(name: 'welcome', source: 'inline'), Tag(name: 'pkm', source: 'inline')],
    ),
    'example.md': Note(
      path: 'example.md', title: 'Example Note',
      frontmatter: Frontmatter(title: 'Example Note', tags: ['example', 'reference'], created: '2026-06-22'),
      body: '# Example Note\n\nThis is an example note showing various Markdown features.\n\n## Links\n\n- Internal: [[Welcome to Stratum]]\n- External: [Stratum Docs](https://stratum.app)\n\n## Tags\n\n#reference #example #demo\n\n## Code\n\n```dart\nvoid main() {\n  print("Hello, Stratum!");\n}\n```\n\n## Tasks\n\n- [x] Set up vault\n- [ ] Create notes\n- [ ] Configure sync\n\nSee also: [[Quantum Computing Overview]] for advanced topics.',
      links: [Link(target: 'Welcome to Stratum', line: 5), Link(target: 'Quantum Computing Overview', line: 18)],
      tags: [Tag(name: 'example', source: 'frontmatter'), Tag(name: 'reference', source: 'frontmatter'), Tag(name: 'reference', source: 'inline'), Tag(name: 'example', source: 'inline'), Tag(name: 'demo', source: 'inline')],
    ),
    'quantum-computing.md': Note(
      path: 'quantum-computing.md', title: 'Quantum Computing Overview',
      frontmatter: Frontmatter(title: 'Quantum Computing Overview', tags: ['quantum', 'computing', 'physics'], aliases: ['QC'], created: '2026-06-20'),
      body: '# Quantum Computing Overview\n\n[[Superposition]] and [[Entanglement]] are the core principles.\n\nQuantum gates operate on [[Qubits]] — see [[Quantum Gate Taxonomy]].\n\n## Key Papers\n\n- [Shor\'s Algorithm](https://arxiv.org/abs/quant-ph/9508027)\n- [[Grover\'s Algorithm]]\n\n#quantum #computing',
      links: [Link(target: 'Superposition', line: 2), Link(target: 'Entanglement', line: 2), Link(target: 'Qubits', line: 4), Link(target: 'Quantum Gate Taxonomy', line: 4), Link(target: "Grover's Algorithm", line: 8)],
      tags: [Tag(name: 'quantum', source: 'frontmatter'), Tag(name: 'computing', source: 'frontmatter'), Tag(name: 'physics', source: 'frontmatter'), Tag(name: 'quantum', source: 'inline'), Tag(name: 'computing', source: 'inline')],
    ),
    'project-todo.md': Note(
      path: 'project-todo.md', title: 'Project TODO',
      frontmatter: Frontmatter(title: 'Project TODO', tags: ['project', 'todo'], created: '2026-06-21'),
      body: '# Project TODO\n\n## Current Sprint\n\n- [ ] Implement graph view [[Quantum Computing Overview]]\n- [x] Set up git sync\n- [ ] Add [[Welcome to Stratum]] documentation\n- [ ] Review [[Example Note]] for accuracy\n\n## Backlog\n\n- [ ] Mobile support\n- [ ] Plugin system\n- [ ] AI chat integration\n\n#project #todo',
      links: [Link(target: 'Quantum Computing Overview', line: 5), Link(target: 'Welcome to Stratum', line: 7), Link(target: 'Example Note', line: 8)],
      tags: [Tag(name: 'project', source: 'frontmatter'), Tag(name: 'todo', source: 'frontmatter'), Tag(name: 'project', source: 'inline'), Tag(name: 'todo', source: 'inline')],
    ),
  };

  // ── Disk I/O ─────────────────────────────────────────────────────

  String _fullPath(String rel) => '$_vaultPath/$rel';

  void _writeToDisk(String path, Note note) {
    if (_vaultPath.isEmpty) return;
    final f = File(_fullPath(path));
    f.parent.createSync(recursive: true);
    final b = StringBuffer();
    final fm = note.frontmatter;
    if (fm.title != null || fm.tags.isNotEmpty) {
      b.writeln('---');
      if (fm.title != null) b.writeln('title: ${fm.title}');
      if (fm.created != null) b.writeln('created: ${fm.created}');
      if (fm.modified != null) b.writeln('modified: ${fm.modified}');
      if (fm.tags.isNotEmpty) b.writeln('tags: [${fm.tags.join(', ')}]');
      if (fm.aliases.isNotEmpty) b.writeln('aliases: [${fm.aliases.join(', ')}]');
      b.writeln('---');
      b.writeln();
    }
    b.write(note.body);
    f.writeAsStringSync(b.toString());
  }

  void _writeAllToDisk() {
    for (final e in _storage.entries) _writeToDisk(e.key, e.value);
  }

  void _loadFromDisk() {
    if (_vaultPath.isEmpty) return;
    final d = Directory(_vaultPath);
    if (!d.existsSync()) return;
    for (final entity in d.listSync(recursive: true)) {
      if (entity is File && entity.path.endsWith('.md')) {
        final rel = entity.path.substring(_vaultPath.length + 1);
        final content = entity.readAsStringSync();
        final parts = _splitFrontmatter(content);
        final fm = parts.fm;
        final body = parts.body;
        final title = fm.title ?? rel.split('/').last.replaceAll('.md', '').replaceAll('-', ' ');
        final links = extractLinks(body);
        final tags = <Tag>[
          ...fm.tags.map((t) => Tag(name: t, source: 'frontmatter')),
          ...extractTags(body),
        ];
        _storage[rel] = Note(
          path: rel, title: title, frontmatter: fm,
          body: body, links: links, tags: tags,
          modifiedAt: entity.lastModifiedSync(),
        );
      }
    }
    _rebuildBacklinks();
  }

  _FmSplit _splitFrontmatter(String content) {
    final lines = content.split('\n');
    if (lines.isEmpty || lines[0].trim() != '---') {
      return _FmSplit(Frontmatter(), content);
    }
    final fmLines = <String>[];
    int i = 1;
    while (i < lines.length && lines[i].trim() != '---') { fmLines.add(lines[i]); i++; }
    if (i < lines.length) i++; // skip closing ---
    String? title, created, modified;
    final tags = <String>[], aliases = <String>[];
    for (final line in fmLines) {
      final m = RegExp(r'^(\w[\w_-]*):\s*(.*)$').firstMatch(line);
      if (m == null) continue;
      final k = m.group(1)!, v = m.group(2)!.trim();
      switch (k) {
        case 'title': title = v; break;
        case 'created': created = v; break;
        case 'modified': modified = v; break;
        case 'tags': tags.addAll(_parseList(v)); break;
        case 'aliases': aliases.addAll(_parseList(v)); break;
      }
    }
    return _FmSplit(Frontmatter(title: title, created: created, modified: modified, tags: tags, aliases: aliases), lines.sublist(i).join('\n'));
  }

  List<String> _parseList(String v) {
    v = v.trim();
    if (v.startsWith('[') && v.endsWith(']')) {
      return v.substring(1, v.length - 1).split(',').map((s) => s.trim().replaceAll('"', '').replaceAll("'", '')).where((s) => s.isNotEmpty).toList();
    }
    return v.split(',').map((s) => s.trim()).where((s) => s.isNotEmpty).toList();
  }

  // ── Interface ────────────────────────────────────────────────────

  @override
  Future<AppConfig> loadConfig() async => AppConfig(vaultPath: _vaultPath);

  @override
  Future<void> saveConfig(AppConfig config) async => _vaultPath = config.vaultPath;

  @override
  Future<List<String>> listNotes({String? tag, String? search}) async {
    _ensureSamples();
    var notes = _storage.keys.toList()..sort();
    if (tag != null) notes = notes.where((p) => _storage[p]!.tags.any((t) => t.name == tag)).toList();
    if (search != null && search.isNotEmpty) {
      final q = search.toLowerCase();
      notes = notes.where((p) {
        final n = _storage[p]!;
        return n.title.toLowerCase().contains(q) || n.body.toLowerCase().contains(q);
      }).toList();
    }
    return notes;
  }

  @override
  Future<Note> openNote(String path) async {
    _ensureSamples();
    if (_storage.containsKey(path)) return _storage[path]!;
    return Note(path: path, title: path.split('/').last.replaceAll('.md', '').replaceAll('-', ' '), frontmatter: Frontmatter(), body: '');
  }

  @override
  Future<void> saveNote(String path, Frontmatter fm, String body) async {
    _ensureSamples();
    final existing = _storage[path];
    final links = extractLinks(body);
    final inlineTags = extractTags(body);
    final fmTags = fm.tags.map((t) => Tag(name: t, source: 'frontmatter')).toList();
    final note = Note(
      path: path,
      title: fm.title ?? existing?.title ?? path.split('/').last.replaceAll('.md', ''),
      frontmatter: fm,
      body: body,
      links: links,
      tags: [...fmTags, ...inlineTags],
      modifiedAt: DateTime.now(),
    );
    _storage[path] = note;
    _writeToDisk(path, note);
    _rebuildBacklinks();
  }

  @override
  Future<void> deleteNote(String path) async {
    _storage.remove(path);
    if (_vaultPath.isNotEmpty) {
      final f = File(_fullPath(path));
      if (f.existsSync()) f.deleteSync();
    }
    _rebuildBacklinks();
  }

  @override
  Future<Note> createNote(String path, {String? title}) async {
    _ensureSamples();
    final t = title ?? path.split('/').last.replaceAll('.md', '').replaceAll('-', ' ');
    final note = Note(
      path: path,
      title: t,
      frontmatter: Frontmatter(title: t, created: DateTime.now().toIso8601String().split('T')[0]),
      body: '',
      modifiedAt: DateTime.now(),
    );
    _storage[path] = note;
    _writeToDisk(path, note);
    return note;
  }

  void _rebuildBacklinks() {
    final blMap = <String, List<Backlink>>{};
    for (final e in _storage.entries) {
      for (final link in e.value.links) {
        final target = _resolveTarget(link.target);
        if (target != null && _storage.containsKey(target) && target != e.key) {
          blMap.putIfAbsent(target, () => []);
          blMap[target]!.add(Backlink(sourcePath: e.key, sourceTitle: e.value.title, contextSnippet: 'Links from [[${link.target}]]', line: link.line));
        }
      }
    }
    for (final e in _storage.entries) {
      _storage[e.key] = Note(
        path: e.value.path, title: e.value.title, frontmatter: e.value.frontmatter,
        body: e.value.body, links: e.value.links, tags: e.value.tags,
        backlinks: blMap[e.key] ?? [], modifiedAt: e.value.modifiedAt,
      );
    }
  }

  String? _resolveTarget(String target) {
    if (_storage.containsKey(target)) return target;
    final tSlug = target.toLowerCase().replaceAll(RegExp(r'[-\s]+'), '-');
    for (final e in _storage.entries) {
      final slug = e.key.split('/').last.replaceAll('.md', '').toLowerCase().replaceAll(RegExp(r'[-\s]+'), '-');
      if (slug == tSlug || e.value.title.toLowerCase() == target.toLowerCase()) return e.key;
    }
    return null;
  }

  @override
  Future<List<SearchResult>> search(String query, {String mode = 'fulltext', int limit = 20}) async {
    _ensureSamples();
    final q = query.toLowerCase();
    final results = <SearchResult>[];
    for (final e in _storage.entries) {
      final n = e.value;
      int score = 0;
      if (n.title.toLowerCase().contains(q)) score += 10;
      if (n.body.toLowerCase().contains(q)) score += 5;
      for (final t in n.tags) { if (t.name.toLowerCase().contains(q)) score += 3; }
      if (score > 0) {
        final snip = _snippet(n.body, q);
        results.add(SearchResult(path: e.key, title: n.title, snippet: snip, score: score.toDouble()));
      }
    }
    results.sort((a, b) => b.score.compareTo(a.score));
    return results.take(limit).toList();
  }

  String _snippet(String body, String q) {
    final lower = body.toLowerCase();
    final idx = lower.indexOf(q);
    if (idx == -1) return body.length > 100 ? '${body.substring(0, 100)}...' : body;
    final start = max(0, idx - 40);
    final end = min(body.length, idx + q.length + 80);
    var s = body.substring(start, end);
    if (start > 0) s = '...$s';
    if (end < body.length) s = '$s...';
    return s;
  }

  @override
  Future<GraphLayout> getGraph() async {
    _ensureSamples();
    final nodes = <GraphNode>[], edges = <GraphEdge>[], edgeSet = <String>{};
    for (final e in _storage.entries) {
      final n = e.value;
      nodes.add(GraphNode(id: n.path, label: n.title, slug: n.slug, tags: n.tags.map((t) => t.name).toSet().toList(), linkCount: n.links.length));
      for (final link in n.links) {
        final tgt = _resolveTarget(link.target);
        if (tgt == null) continue;
        final key = '${n.path}->$tgt';
        if (!edgeSet.contains(key)) { edgeSet.add(key); edges.add(GraphEdge(source: n.path, target: tgt)); }
      }
    }
    return GraphLayout(nodes: nodes, edges: edges);
  }

  @override
  Future<List<TagCount>> getTagCloud() async {
    _ensureSamples();
    final counts = <String, int>{};
    for (final n in _storage.values) {
      for (final t in n.tags) { counts[t.name] = (counts[t.name] ?? 0) + 1; }
    }
    return counts.entries.map((e) => TagCount(name: e.key, count: e.value)).toList()..sort((a, b) => b.count.compareTo(a.count));
  }

  @override
  Future<SyncStatus> getSyncStatus() async => SyncStatus(status: 'up_to_date', lastSync: DateTime.now().toIso8601String());

  @override
  Future<void> syncVault() async => await Future.delayed(const Duration(seconds: 1));

  @override
  Future<VaultStats> getStats() async {
    _ensureSamples();
    int links = 0;
    for (final n in _storage.values) { links += n.links.length; }
    return VaultStats(noteCount: _storage.length, tagCount: (await getTagCloud()).length, linkCount: links, totalSizeBytes: _storage.values.fold<int>(0, (s, n) => s + n.body.length));
  }

  @override
  Future<String> askAi(String q) async => 'Mock AI response to: "$q"\n\nMock backend for development.';
}

class _FmSplit {
  final Frontmatter fm;
  final String body;
  _FmSplit(this.fm, this.body);
}
