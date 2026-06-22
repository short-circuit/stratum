import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/config.dart';

class SettingsProvider extends ChangeNotifier {
  AppConfig _config = AppConfig();
  bool _loaded = false;

  AppConfig get config => _config;
  bool get loaded => _loaded;

  Future<void> loadSettings() async {
    final prefs = await SharedPreferences.getInstance();
    _config = AppConfig(
      vaultPath: prefs.getString('vault_path') ?? '',
      sync: SyncConfig(
        mode: prefs.getString('sync_mode') ?? 'manual',
        remoteUrl: prefs.getString('sync_remote_url'),
        branch: prefs.getString('sync_branch') ?? 'main',
        autoCommitIntervalSecs: prefs.getInt('sync_auto_commit_interval') ?? 300,
      ),
      theme: ThemeConfig(
        darkMode: prefs.getBool('theme_dark_mode') ?? true,
        fontSize: prefs.getDouble('theme_font_size') ?? 16,
        fontFamily: prefs.getString('theme_font_family') ?? 'Inter, sans-serif',
        showLineNumbers: prefs.getBool('theme_show_line_numbers') ?? false,
        sidebarWidth: prefs.getDouble('theme_sidebar_width') ?? 280,
      ),
      ai: AiConfig(
        provider: prefs.getString('ai_provider') ?? 'Ollama',
        endpoint: prefs.getString('ai_endpoint'),
        apiKey: prefs.getString('ai_api_key'),
        model: prefs.getString('ai_model') ?? 'llama3.2',
        ragEnabled: prefs.getBool('ai_rag_enabled') ?? true,
      ),
      watcherEnabled: prefs.getBool('watcher_enabled') ?? true,
      watcherDebounceMs: prefs.getInt('watcher_debounce_ms') ?? 500,
    );
    _loaded = true;
    notifyListeners();
  }

  ThemeConfig get theme => _config.theme;
  SyncConfig get sync => _config.sync;
  AiConfig get ai => _config.ai;

  Future<void> _persist() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('vault_path', _config.vaultPath);
    await prefs.setString('sync_mode', _config.sync.mode);
    if (_config.sync.remoteUrl != null) {
      await prefs.setString('sync_remote_url', _config.sync.remoteUrl!);
    }
    await prefs.setString('sync_branch', _config.sync.branch);
    await prefs.setInt('sync_auto_commit_interval', _config.sync.autoCommitIntervalSecs);
    await prefs.setBool('theme_dark_mode', _config.theme.darkMode);
    await prefs.setDouble('theme_font_size', _config.theme.fontSize);
    await prefs.setString('theme_font_family', _config.theme.fontFamily);
    await prefs.setBool('theme_show_line_numbers', _config.theme.showLineNumbers);
    await prefs.setDouble('theme_sidebar_width', _config.theme.sidebarWidth);
    await prefs.setString('ai_provider', _config.ai.provider);
    if (_config.ai.endpoint != null) {
      await prefs.setString('ai_endpoint', _config.ai.endpoint!);
    }
    if (_config.ai.apiKey != null) {
      await prefs.setString('ai_api_key', _config.ai.apiKey!);
    }
    await prefs.setString('ai_model', _config.ai.model);
    await prefs.setBool('ai_rag_enabled', _config.ai.ragEnabled);
    await prefs.setBool('watcher_enabled', _config.watcherEnabled);
    await prefs.setInt('watcher_debounce_ms', _config.watcherDebounceMs);
  }

  Future<void> setDarkMode(bool value) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: ThemeConfig(
        darkMode: value,
        fontSize: _config.theme.fontSize,
        fontFamily: _config.theme.fontFamily,
        showLineNumbers: _config.theme.showLineNumbers,
        sidebarWidth: _config.theme.sidebarWidth,
      ),
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setFontSize(double value) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: ThemeConfig(
        darkMode: _config.theme.darkMode,
        fontSize: value,
        fontFamily: _config.theme.fontFamily,
        showLineNumbers: _config.theme.showLineNumbers,
        sidebarWidth: _config.theme.sidebarWidth,
      ),
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setShowLineNumbers(bool value) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: ThemeConfig(
        darkMode: _config.theme.darkMode,
        fontSize: _config.theme.fontSize,
        fontFamily: _config.theme.fontFamily,
        showLineNumbers: value,
        sidebarWidth: _config.theme.sidebarWidth,
      ),
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setSyncMode(String mode) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: SyncConfig(
        mode: mode,
        remoteUrl: _config.sync.remoteUrl,
        branch: _config.sync.branch,
        autoCommitIntervalSecs: _config.sync.autoCommitIntervalSecs,
      ),
      theme: _config.theme,
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setAiProvider(String provider) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: _config.theme,
      ai: AiConfig(
        provider: provider,
        endpoint: _config.ai.endpoint,
        model: _config.ai.model,
        ragEnabled: _config.ai.ragEnabled,
      ),
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setAiModel(String model) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: _config.theme,
      ai: AiConfig(
        provider: _config.ai.provider,
        endpoint: _config.ai.endpoint,
        model: model,
        ragEnabled: _config.ai.ragEnabled,
      ),
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setRagEnabled(bool value) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: _config.theme,
      ai: AiConfig(
        provider: _config.ai.provider,
        endpoint: _config.ai.endpoint,
        model: _config.ai.model,
        ragEnabled: value,
      ),
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setVaultPath(String path) async {
    _config = AppConfig(
      vaultPath: path,
      sync: _config.sync,
      theme: _config.theme,
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setRemoteUrl(String url) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: SyncConfig(
        mode: _config.sync.mode,
        remoteUrl: url,
        branch: _config.sync.branch,
        autoCommitIntervalSecs: _config.sync.autoCommitIntervalSecs,
      ),
      theme: _config.theme,
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setEndpoint(String endpoint) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: _config.theme,
      ai: AiConfig(
        provider: _config.ai.provider,
        endpoint: endpoint,
        apiKey: _config.ai.apiKey,
        model: _config.ai.model,
        ragEnabled: _config.ai.ragEnabled,
      ),
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setApiKey(String key) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: _config.sync,
      theme: _config.theme,
      ai: AiConfig(
        provider: _config.ai.provider,
        endpoint: _config.ai.endpoint,
        apiKey: key,
        model: _config.ai.model,
        ragEnabled: _config.ai.ragEnabled,
      ),
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  Future<void> setAutoCommitInterval(int secs) async {
    _config = AppConfig(
      vaultPath: _config.vaultPath,
      sync: SyncConfig(
        mode: _config.sync.mode,
        remoteUrl: _config.sync.remoteUrl,
        branch: _config.sync.branch,
        autoCommitIntervalSecs: secs,
      ),
      theme: _config.theme,
      ai: _config.ai,
      watcherEnabled: _config.watcherEnabled,
      watcherDebounceMs: _config.watcherDebounceMs,
    );
    await _persist();
    notifyListeners();
  }

  bool get isFirstLaunch => _config.vaultPath.isEmpty;
}
