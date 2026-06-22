
import 'package:flutter/foundation.dart';
import '../services/rust_backend.dart';
import '../services/mock_backend.dart';

class SearchProvider extends ChangeNotifier {
  final RustBackend _backend = MockBackend();

  String _query = '';
  String _mode = 'fulltext';
  List<SearchResult> _results = [];
  bool _searching = false;

  String get query => _query;
  String get mode => _mode;
  List<SearchResult> get results => _results;
  bool get searching => _searching;
  bool get hasResults => _results.isNotEmpty;

  Future<void> search(String q, {String mode = 'fulltext'}) async {
    _query = q;
    _mode = mode;
    if (q.trim().isEmpty) { _results = []; notifyListeners(); return; }
    _searching = true;
    notifyListeners();
    try {
      _results = await _backend.search(q, mode: mode);
    } catch (e) {
      _results = [];
    }
    _searching = false;
    notifyListeners();
  }

  void setMode(String mode) {
    _mode = mode;
    if (_query.isNotEmpty) search(_query, mode: mode);
  }

  void clear() {
    _query = '';
    _results = [];
    _searching = false;
    notifyListeners();
  }
}
