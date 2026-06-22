


class AppConfig {
  final String vaultPath;
  final SyncConfig sync;
  final ThemeConfig theme;
  final AiConfig ai;
  final bool watcherEnabled;
  final int watcherDebounceMs;

  AppConfig({
    this.vaultPath = '',
    SyncConfig? sync,
    ThemeConfig? theme,
    AiConfig? ai,
    this.watcherEnabled = true,
    this.watcherDebounceMs = 500,
  }) : sync = sync ?? SyncConfig(),
       theme = theme ?? ThemeConfig(),
       ai = ai ?? AiConfig();

  Map<String, dynamic> toJson() => {
    'vault_path': vaultPath,
    'sync': sync.toJson(),
    'theme': theme.toJson(),
    'ai': ai.toJson(),
    'watcher': {'enabled': watcherEnabled, 'debounce_ms': watcherDebounceMs},
  };

  factory AppConfig.fromJson(Map<String, dynamic> json) => AppConfig(
    vaultPath: json['vault_path'] as String? ?? '',
    sync: SyncConfig.fromJson(json['sync'] as Map<String, dynamic>? ?? {}),
    theme: ThemeConfig.fromJson(json['theme'] as Map<String, dynamic>? ?? {}),
    ai: AiConfig.fromJson(json['ai'] as Map<String, dynamic>? ?? {}),
    watcherEnabled: (json['watcher'] as Map<String, dynamic>?)?['enabled'] as bool? ?? true,
    watcherDebounceMs: (json['watcher'] as Map<String, dynamic>?)?['debounce_ms'] as int? ?? 500,
  );
}

class SyncConfig {
  final String mode; // manual, auto_commit, auto_sync, background
  final String? remoteUrl;
  final String branch;
  final int autoCommitIntervalSecs;

  SyncConfig({this.mode = 'manual', this.remoteUrl, this.branch = 'main', this.autoCommitIntervalSecs = 300});

  Map<String, dynamic> toJson() => {
    'mode': mode, 'remote_url': remoteUrl, 'branch': branch, 'auto_commit_interval_secs': autoCommitIntervalSecs,
  };

  factory SyncConfig.fromJson(Map<String, dynamic> json) => SyncConfig(
    mode: json['mode'] as String? ?? 'manual',
    remoteUrl: json['remote_url'] as String?,
    branch: json['branch'] as String? ?? 'main',
    autoCommitIntervalSecs: json['auto_commit_interval_secs'] as int? ?? 300,
  );
}

class ThemeConfig {
  final bool darkMode;
  final double fontSize;
  final String fontFamily;
  final bool showLineNumbers;
  final double sidebarWidth;

  ThemeConfig({this.darkMode = true, this.fontSize = 16, this.fontFamily = 'Inter, sans-serif', this.showLineNumbers = false, this.sidebarWidth = 280});

  Map<String, dynamic> toJson() => {
    'dark_mode': darkMode, 'font_size': fontSize, 'font_family': fontFamily, 'show_line_numbers': showLineNumbers, 'sidebar_width': sidebarWidth,
  };

  factory ThemeConfig.fromJson(Map<String, dynamic> json) => ThemeConfig(
    darkMode: json['dark_mode'] as bool? ?? true,
    fontSize: (json['font_size'] as num?)?.toDouble() ?? 16,
    fontFamily: json['font_family'] as String? ?? 'Inter, sans-serif',
    showLineNumbers: json['show_line_numbers'] as bool? ?? false,
    sidebarWidth: (json['sidebar_width'] as num?)?.toDouble() ?? 280,
  );
}

class AiConfig {
  final String provider;
  final String? endpoint;
  final String? apiKey;
  final String model;
  final bool ragEnabled;

  AiConfig({this.provider = 'Ollama', this.endpoint, this.apiKey, this.model = 'llama3.2', this.ragEnabled = true});

  String get defaultEndpoint {
    switch (provider) {
      case 'Ollama': return endpoint ?? 'http://localhost:11434';
      case 'OpenAI': return endpoint ?? 'https://api.openai.com/v1';
      case 'Anthropic': return endpoint ?? 'https://api.anthropic.com/v1';
      default: return endpoint ?? '';
    }
  }

  Map<String, dynamic> toJson() => {
    'provider': provider, 'endpoint': endpoint, 'api_key': apiKey, 'model': model, 'rag_enabled': ragEnabled,
  };

  factory AiConfig.fromJson(Map<String, dynamic> json) => AiConfig(
    provider: json['provider'] as String? ?? 'Ollama',
    endpoint: json['endpoint'] as String?,
    apiKey: json['api_key'] as String?,
    model: json['model'] as String? ?? 'llama3.2',
    ragEnabled: json['rag_enabled'] as bool? ?? true,
  );
}
