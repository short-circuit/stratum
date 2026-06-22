
import 'package:flutter/foundation.dart';
import '../services/rust_backend.dart';
import '../services/mock_backend.dart';

class SyncProvider extends ChangeNotifier {
  final RustBackend _backend = MockBackend();
  SyncStatus _status = SyncStatus();
  bool _syncing = false;

  SyncStatus get status => _status;
  bool get syncing => _syncing;
  bool get hasConflicts => _status.conflictFiles.isNotEmpty;
  String get statusLabel {
    switch (_status.status) {
      case 'idle': return 'Sync idle';
      case 'syncing': return 'Syncing...';
      case 'conflicts': return '${_status.conflictFiles.length} conflict(s)';
      case 'error': return 'Sync error';
      case 'up_to_date': return 'Up to date';
      default: return 'Unknown';
    }
  }

  Future<void> loadStatus() async {
    try {
      _status = await _backend.getSyncStatus();
      notifyListeners();
    } catch (_) {}
  }

  Future<void> sync() async {
    _syncing = true;
    _status = SyncStatus(status: 'syncing');
    notifyListeners();
    try {
      await _backend.syncVault();
      _status = SyncStatus(status: 'up_to_date', lastSync: DateTime.now().toIso8601String());
    } catch (e) {
      _status = SyncStatus(status: 'error');
    }
    _syncing = false;
    notifyListeners();
  }
}
