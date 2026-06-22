import 'dart:convert';
import 'dart:math';
import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:http/http.dart' as http;
import 'package:provider/provider.dart';
import '../models/config.dart';
import '../providers/settings_provider.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  List<String> _fetchedModels = [];
  bool _fetching = false;
  String? _fetchErr;

  @override
  Widget build(BuildContext context) {
    final s = context.watch<SettingsProvider>();
    final c = s.config;
    final ai = c.ai;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
        leading: IconButton(icon: const Icon(Icons.arrow_back), onPressed: () => Navigator.pop(context)),
      ),
      body: ListView(children: [
        _sec('Vault'),
        _row(Icons.folder, 'Vault Path', c.vaultPath.isEmpty ? 'Not configured' : c.vaultPath, const Icon(Icons.folder_open, size: 18), () => _pickFolder(s)),
        _row(Icons.cloud_sync, 'Sync Mode', c.sync.mode.replaceAll('_', ' '), null, () => _syncModePicker(s, c.sync.mode)),
        _row(Icons.link, 'Remote URL', c.sync.remoteUrl ?? 'Git remote for push/pull sync', null, () => _editDlg('Remote URL', c.sync.remoteUrl ?? '', 'git@github.com:user/vault.git', s.setRemoteUrl)),
        const Divider(),
        _sec('Appearance'),
        SwitchListTile(
          secondary: Icon(Icons.dark_mode, color: c.theme.darkMode ? Colors.amber[300] : Colors.grey),
          title: const Text('Dark Mode'),
          value: c.theme.darkMode,
          onChanged: (v) => s.setDarkMode(v),
        ),
        _row(Icons.text_fields, 'Font Size', c.theme.fontSize.toInt().toString(), null, () => _fontPicker(s, c.theme.fontSize)),
        SwitchListTile(
          secondary: const Icon(Icons.line_weight),
          title: const Text('Show Line Numbers'),
          value: c.theme.showLineNumbers,
          onChanged: (v) => s.setShowLineNumbers(v),
        ),
        const Divider(),
        _sec('AI'),
        _row(Icons.psychology, 'Provider', ai.provider, null, () => _providerPicker(s, ai.provider)),
        _row(Icons.settings_ethernet, 'Endpoint URL', ai.endpoint ?? ai.defaultEndpoint, null, () => _editDlg('Endpoint URL', ai.endpoint ?? ai.defaultEndpoint, ai.defaultEndpoint, s.setEndpoint)),
        if (ai.provider != 'Ollama')
          _row(Icons.key, 'API Key', ai.apiKey != null ? '••••${ai.apiKey!.substring(max(0, ai.apiKey!.length - 4))}' : 'Not set', null, () => _editDlg('API Key', ai.apiKey ?? '', 'API key for $ai.provider', s.setApiKey, obscure: true)),
        ListTile(
          leading: const Icon(Icons.language, size: 20),
          title: const Text('Model', style: TextStyle(fontSize: 15)),
          subtitle: _fetchErr != null
              ? Text(_fetchErr!, style: TextStyle(fontSize: 12, color: Colors.red[400]))
              : Text(ai.model, style: TextStyle(fontSize: 12, color: Colors.grey[500])),
          trailing: _fetching
              ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
              : const Icon(Icons.chevron_right, size: 18),
          onTap: () => _modelPicker(s, ai),
        ),
        SwitchListTile(
          secondary: const Icon(Icons.troubleshoot),
          title: const Text('Enable RAG'),
          value: ai.ragEnabled,
          onChanged: (v) => s.setRagEnabled(v),
        ),
        const Divider(),
        _sec('Sync'),
        _row(Icons.timer, 'Auto-commit interval', '${c.sync.autoCommitIntervalSecs ~/ 60} min', null, () => _intervalPicker(s, c.sync.autoCommitIntervalSecs)),
        const Divider(),
        _sec('About'),
        _row(Icons.info, 'Version', '0.1.0', null, () {}),
        _row(Icons.description, 'License', 'AGPL-3.0', null, () {}),
        const SizedBox(height: 40),
      ]),
    );
  }

  // ── Folder picker ────────────────────────────────────────────────

  Future<void> _pickFolder(SettingsProvider s) async {
    final r = await FilePicker.platform.getDirectoryPath();
    if (r != null) s.setVaultPath(r);
  }

  // ── Model fetch ──────────────────────────────────────────────────

  Future<void> _fetchModels(String endpoint) async {
    setState(() { _fetching = true; _fetchErr = null; });
    try {
      final base = endpoint.endsWith('/') ? endpoint.substring(0, endpoint.length - 1) : endpoint;
      final uri = Uri.parse('$base/api/tags');
      final resp = await http.get(uri).timeout(const Duration(seconds: 6));
      if (resp.statusCode == 200) {
        final data = jsonDecode(resp.body);
        final models = <String>[];
        if (data is Map && data['models'] is List) {
          for (final m in data['models']) {
            if (m is Map) {
              final n = m['name'];
              if (n is String) models.add(n);
            }
          }
        }
        if (models.isEmpty) throw 'No models returned';
        setState(() => _fetchedModels = models);
      } else {
        setState(() => _fetchedModels = []);
        _fetchErr = 'HTTP ${resp.statusCode}';
      }
    } catch (e) {
      setState(() { _fetchedModels = []; _fetchErr = 'Cannot reach Ollama'; });
    }
    setState(() => _fetching = false);
  }

  // ── Dialogs ──────────────────────────────────────────────────────

  void _editDlg(String title, String cur, String hint, ValueChanged<String> onSave, {bool obscure = false}) {
    final ctrl = TextEditingController(text: cur);
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: TextField(controller: ctrl, autofocus: true, obscureText: obscure, decoration: InputDecoration(hintText: hint)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(onPressed: () { onSave(ctrl.text.trim()); Navigator.pop(ctx); }, child: const Text('Save')),
        ],
      ),
    );
  }

  void _syncModePicker(SettingsProvider s, String cur) {
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('Sync Mode'),
        children: ['manual', 'auto_commit', 'auto_sync', 'background'].map((m) => RadioListTile<String>(
          title: Text(m.replaceAll('_', ' ')),
          value: m, groupValue: cur,
          onChanged: (v) { if (v != null) s.setSyncMode(v); Navigator.pop(ctx); },
        )).toList(),
      ),
    );
  }

  void _providerPicker(SettingsProvider s, String cur) {
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('AI Provider'),
        children: ['Ollama', 'OpenAI', 'Anthropic', 'Custom'].map((p) => RadioListTile<String>(
          title: Text(p),
          value: p, groupValue: cur,
          onChanged: (v) { if (v != null) s.setAiProvider(v); Navigator.pop(ctx); },
        )).toList(),
      ),
    );
  }

  void _modelPicker(SettingsProvider s, AiConfig ai) async {
    // Fetch if needed
    if (_fetchedModels.isEmpty && ai.provider == 'Ollama') {
      await _fetchModels(ai.endpoint ?? ai.defaultEndpoint);
    }

    if (!mounted) return;

    final defaults = <String>{
      if (ai.model.isNotEmpty) ai.model,
      if (ai.provider == 'Ollama') ...['llama3.2', 'llama3.1', 'mistral', 'codellama', 'mixtral'],
      if (ai.provider == 'OpenAI') ...['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
      if (ai.provider == 'Anthropic') ...['claude-3-5-sonnet-20241022', 'claude-3-opus-20240229', 'claude-3-haiku-20240307'],
      ..._fetchedModels,
    };

    final unique = defaults.toSet().toList();

    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: Row(children: [
          const Text('Model'),
          const Spacer(),
          if (ai.provider == 'Ollama')
            TextButton.icon(
              onPressed: () async {
                Navigator.pop(ctx);
                await _fetchModels(ai.endpoint ?? ai.defaultEndpoint);
                if (mounted) _modelPicker(s, ai);
              },
              icon: const Icon(Icons.refresh, size: 16),
              label: const Text('Fetch', style: TextStyle(fontSize: 12)),
            ),
        ]),
        children: unique.map((m) => ListTile(
          title: Text(m, style: const TextStyle(fontSize: 14)),
          leading: Icon(
            m == ai.model ? Icons.radio_button_checked : Icons.radio_button_off,
            color: m == ai.model ? Theme.of(context).colorScheme.primary : Colors.grey,
            size: 18,
          ),
          onTap: () {
            s.setAiModel(m);
            Navigator.pop(ctx);
          },
        )).toList(),
      ),
    );
  }

  void _fontPicker(SettingsProvider s, double cur) {
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('Font Size'),
        children: [12, 14, 16, 18, 20, 24].map((sz) => RadioListTile<double>(
          title: Text('$sz'),
          value: sz.toDouble(), groupValue: cur,
          onChanged: (v) { if (v != null) s.setFontSize(v); Navigator.pop(ctx); },
        )).toList(),
      ),
    );
  }

  void _intervalPicker(SettingsProvider s, int cur) {
    final opts = [1, 2, 5, 10, 15, 30, 60];
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('Auto-commit Interval'),
        children: opts.map((m) {
          final secs = m * 60;
          return RadioListTile<int>(
            title: Text(m == 1 ? '1 minute' : '$m minutes'),
            value: secs, groupValue: cur,
            onChanged: (v) { if (v != null) s.setAutoCommitInterval(v); Navigator.pop(ctx); },
          );
        }).toList(),
      ),
    );
  }

  // ── Helpers ──────────────────────────────────────────────────────

  Widget _sec(String t) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 20, 16, 8),
    child: Text(t, style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13, color: Colors.grey[500])),
  );

  Widget _row(IconData icon, String title, String sub, Widget? trailing, VoidCallback onTap) => ListTile(
    leading: Icon(icon, size: 20),
    title: Text(title, style: const TextStyle(fontSize: 15)),
    subtitle: Text(sub, style: TextStyle(fontSize: 12, color: Colors.grey[500])),
    trailing: trailing ?? const Icon(Icons.chevron_right, size: 18),
    onTap: onTap,
  );
}
