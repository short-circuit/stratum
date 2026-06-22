
import '../models/note.dart';
import '../models/graph.dart';
import '../models/config.dart';

/// Abstract interface for the Rust core backend.
/// In production, this calls into Rust via flutter_rust_bridge FFI.
/// During development, use MockBackend for testing without native builds.
abstract class RustBackend {
  Future<AppConfig> loadConfig();
  Future<void> saveConfig(AppConfig config);

  // Vault
  Future<List<String>> listNotes({String? tag, String? search});
  Future<Note> openNote(String path);
  Future<void> saveNote(String path, Frontmatter frontmatter, String body);
  Future<void> deleteNote(String path);
  Future<Note> createNote(String path, {String? title});

  // Search
  Future<List<SearchResult>> search(String query, {String mode = 'fulltext', int limit = 20});

  // Graph
  Future<GraphLayout> getGraph();

  // Tags
  Future<List<TagCount>> getTagCloud();

  // Sync
  Future<SyncStatus> getSyncStatus();
  Future<void> syncVault();

  // Stats
  Future<VaultStats> getStats();

  // AI
  Future<String> askAi(String question);
}

class SearchResult {
  final String path;
  final String title;
  final String snippet;
  final double score;
  SearchResult({required this.path, required this.title, this.snippet = '', this.score = 0});
}

class TagCount {
  final String name;
  final int count;
  TagCount({required this.name, required this.count});
}

class SyncStatus {
  final String status; // idle, syncing, conflicts, error, up_to_date
  final String? lastSync;
  final int pendingCommits;
  final List<String> conflictFiles;
  SyncStatus({this.status = 'idle', this.lastSync, this.pendingCommits = 0, this.conflictFiles = const []});
}

class VaultStats {
  final int noteCount;
  final int tagCount;
  final int linkCount;
  final int totalSizeBytes;
  VaultStats({this.noteCount = 0, this.tagCount = 0, this.linkCount = 0, this.totalSizeBytes = 0});
}
