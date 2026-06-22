import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:crypto/crypto.dart';

class AssetRef {
  final String assetPath;  // path within vault: assets/<hash>.<ext>
  final String originalName;
  final String hash;       // SHA-256 hex
  final int sizeBytes;

  AssetRef({
    required this.assetPath,
    required this.originalName,
    required this.hash,
    required this.sizeBytes,
  });

  Map<String, dynamic> toJson() => {
    'asset_path': assetPath,
    'original_name': originalName,
    'hash': hash,
    'size_bytes': sizeBytes,
  };

  factory AssetRef.fromJson(Map<String, dynamic> json) => AssetRef(
    assetPath: json['asset_path'] as String,
    originalName: json['original_name'] as String,
    hash: json['hash'] as String,
    sizeBytes: json['size_bytes'] as int,
  );
}

/// Manages asset files within a vault.
/// Assets are stored in <vault>/assets/ with hash-based filenames to
/// deduplicate identical files. A manifest at assets/.manifest.json
/// tracks which notes reference which assets.
class AssetManager {
  final String vaultPath;

  AssetManager(this.vaultPath);

  String get assetsDir => '$vaultPath/assets';
  String get manifestPath => '$assetsDir/.manifest.json';

  /// Ensure assets directory and manifest exist.
  Future<void> init() async {
    final dir = Directory(assetsDir);
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    final manifestFile = File(manifestPath);
    if (!await manifestFile.exists()) {
      await manifestFile.writeAsString('{}');
    }
  }

  /// Import an asset file from [sourcePath] into the vault.
  /// Returns the relative asset path (e.g. "assets/abc123.png") for
  /// embedding in markdown, or null if the file doesn't exist.
  /// If an asset with the same hash already exists, returns the
  /// existing path without copying.
  Future<AssetRef?> importFile(String sourcePath) async {
    await init();
    final srcFile = File(sourcePath);
    if (!await srcFile.exists()) return null;

    final bytes = await srcFile.readAsBytes();
    return _importBytes(bytes, sourcePath.split('/').last);
  }

  /// Import asset from raw bytes with an original filename.
  Future<AssetRef> _importBytes(Uint8List bytes, String originalName) async {
    final hash = sha256.convert(bytes).toString();
    final ext = originalName.contains('.') ? originalName.substring(originalName.lastIndexOf('.')) : '';
    final assetFilename = '$hash$ext';
    final assetPath = 'assets/$assetFilename';
    final destFile = File('$assetsDir/$assetFilename');

    // Check if this hash already exists
    if (!await destFile.exists()) {
      await destFile.writeAsBytes(bytes);
    }

    final ref = AssetRef(
      assetPath: assetPath,
      originalName: originalName,
      hash: hash,
      sizeBytes: bytes.length,
    );

    // Update manifest
    await _updateManifest(ref);
    return ref;
  }

  /// Add an asset reference to a specific note in the manifest.
  Future<void> addRefForNote(String notePath, String assetAssetPath) async {
    await init();
    final manifestFile = File(manifestPath);
    final json = jsonDecode(await manifestFile.readAsString()) as Map<String, dynamic>;
    final refs = (json[notePath] as List?)?.cast<String>() ?? [];
    if (!refs.contains(assetAssetPath)) {
      refs.add(assetAssetPath);
      json[notePath] = refs;
      await manifestFile.writeAsString(const JsonEncoder.withIndent('  ').convert(json));
    }
  }

  /// Get all assets referenced by a note.
  Future<List<AssetRef>> getNoteAssets(String notePath) async {
    await init();
    final manifestFile = File(manifestPath);
    final json = jsonDecode(await manifestFile.readAsString()) as Map<String, dynamic>;
    final refs = (json[notePath] as List?)?.cast<String>() ?? [];
    return refs.map((path) {
      final file = File('$vaultPath/$path');
      return AssetRef(
        assetPath: path,
        originalName: path.split('/').last,
        hash: path.split('/').last.split('.').first,
        sizeBytes: file.existsSync() ? file.lengthSync() : 0,
      );
    }).toList();
  }

  /// Find orphaned assets (not referenced by any note).
  /// Returns list of asset paths that can be safely deleted.
  Future<List<String>> findOrphans() async {
    await init();
    final manifestFile = File(manifestPath);
    final json = jsonDecode(await manifestFile.readAsString()) as Map<String, dynamic>;
    final referenced = <String>{};
    for (final entry in json.entries) {
      for (final ref in (entry.value as List).cast<String>()) {
        referenced.add(ref);
      }
    }
    final orphans = <String>[];
    final dir = Directory(assetsDir);
    if (await dir.exists()) {
      await for (final entity in dir.list()) {
        if (entity is File && !entity.path.endsWith('.manifest.json')) {
          final relPath = 'assets/${entity.path.split('/').last}';
          if (!referenced.contains(relPath)) {
            orphans.add(relPath);
          }
        }
      }
    }
    return orphans;
  }

  /// Compute SHA-256 hash of a file without importing it.
  Future<String> computeHash(String filePath) async {
    final file = File(filePath);
    final bytes = await file.readAsBytes();
    return sha256.convert(bytes).toString();
  }

  /// Check if an asset hash already exists in the vault.
  Future<bool> hashExists(String hash) async {
    final dir = Directory(assetsDir);
    if (!await dir.exists()) return false;
    await for (final entity in dir.list()) {
      if (entity is File) {
        final name = entity.path.split('/').last;
        if (name.startsWith(hash)) return true;
      }
    }
    return false;
  }

  Future<void> _updateManifest(AssetRef ref) async {
    final manifestFile = File(manifestPath);
    final json = jsonDecode(await manifestFile.readAsString()) as Map<String, dynamic>;
    json[ref.hash] = ref.toJson();
    await manifestFile.writeAsString(const JsonEncoder.withIndent('  ').convert(json));
  }
}
