import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:provider/provider.dart';
import '../providers/settings_provider.dart';

class WizardScreen extends StatefulWidget {
  const WizardScreen({super.key});

  @override
  State<WizardScreen> createState() => _WizardScreenState();
}

class _WizardScreenState extends State<WizardScreen> {
  final _pageCtrl = PageController();
  int _page = 0;

  @override
  void dispose() {
    _pageCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final settings = context.watch<SettingsProvider>();

    return Scaffold(
      body: Column(children: [
        // Progress indicator
        Padding(
          padding: const EdgeInsets.only(top: 48, left: 24, right: 24),
          child: Row(children: [
            for (int i = 0; i < 3; i++)
              Expanded(
                child: Container(
                  height: 4,
                  margin: const EdgeInsets.symmetric(horizontal: 2),
                  decoration: BoxDecoration(
                    color: i <= _page ? Theme.of(context).colorScheme.primary : Colors.grey[300],
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
          ]),
        ),
        Expanded(
          child: PageView(
            controller: _pageCtrl,
            onPageChanged: (p) => setState(() => _page = p),
            children: [
              _buildWelcome(context),
              _buildVaultPath(context, settings),
              _buildTheme(context, settings),
            ],
          ),
        ),
        // Bottom buttons
        Padding(
          padding: const EdgeInsets.all(24),
          child: Row(children: [
            if (_page > 0)
              TextButton(onPressed: () => _pageCtrl.previousPage(duration: const Duration(milliseconds: 300), curve: Curves.easeInOut), child: const Text('Back')),
            const Spacer(),
            FilledButton(
              onPressed: () {
                if (_page < 2) {
                  _pageCtrl.nextPage(duration: const Duration(milliseconds: 300), curve: Curves.easeInOut);
                } else {
                  Navigator.pushReplacementNamed(context, '/');
                }
              },
              child: Text(_page < 2 ? 'Next' : 'Get Started'),
            ),
          ]),
        ),
      ]),
    );
  }

  Widget _buildWelcome(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 40),
      child: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
        Icon(Icons.auto_stories, size: 80, color: Theme.of(context).colorScheme.primary),
        const SizedBox(height: 24),
        Text('Welcome to Stratum', style: Theme.of(context).textTheme.headlineMedium?.copyWith(fontWeight: FontWeight.w700)),
        const SizedBox(height: 16),
        Text(
          'Your privacy-first, offline-capable personal knowledge management system.\n\nNotes are plain Markdown files — zero vendor lock-in.',
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 15, color: Colors.grey[600], height: 1.6),
        ),
      ]),
    );
  }

  Widget _buildVaultPath(BuildContext context, SettingsProvider settings) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 40),
      child: Column(mainAxisAlignment: MainAxisAlignment.center, crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text('Choose a Vault Location', style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w600)),
        const SizedBox(height: 12),
        Text('Your notes will be stored as plain .md files in this folder.', style: TextStyle(fontSize: 14, color: Colors.grey[600])),
        const SizedBox(height: 24),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            border: Border.all(color: Colors.grey[300]!),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Row(children: [
            Expanded(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text('Vault Path', style: TextStyle(fontSize: 12, color: Colors.grey[500])),
                const SizedBox(height: 4),
                Text(
                  settings.config.vaultPath.isEmpty ? 'Not selected' : settings.config.vaultPath,
                  style: TextStyle(fontSize: 14, fontWeight: settings.config.vaultPath.isEmpty ? FontWeight.normal : FontWeight.w500),
                  overflow: TextOverflow.ellipsis,
                ),
              ]),
            ),
            const SizedBox(width: 12),
            FilledButton.tonalIcon(
              onPressed: () async {
                final result = await FilePicker.platform.getDirectoryPath();
                if (result != null) settings.setVaultPath(result);
              },
              icon: const Icon(Icons.folder_open, size: 18),
              label: const Text('Browse'),
            ),
          ]),
        ),
      ]),
    );
  }

  Widget _buildTheme(BuildContext context, SettingsProvider settings) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 40),
      child: Column(mainAxisAlignment: MainAxisAlignment.center, crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text('Appearance', style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w600)),
        const SizedBox(height: 12),
        Text('Choose your preferred theme.', style: TextStyle(fontSize: 14, color: Colors.grey[600])),
        const SizedBox(height: 24),
        Row(children: [
          Expanded(
            child: _themeCard(context, 'Light Mode', Icons.light_mode, !settings.theme.darkMode, () => settings.setDarkMode(false)),
          ),
          const SizedBox(width: 16),
          Expanded(
            child: _themeCard(context, 'Dark Mode', Icons.dark_mode, settings.theme.darkMode, () => settings.setDarkMode(true)),
          ),
        ]),
      ]),
    );
  }

  Widget _themeCard(BuildContext context, String label, IconData icon, bool selected, VoidCallback onTap) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 32),
        decoration: BoxDecoration(
          color: selected ? Theme.of(context).colorScheme.primaryContainer : null,
          border: Border.all(color: selected ? Theme.of(context).colorScheme.primary : Colors.grey[300]!, width: selected ? 2 : 1),
          borderRadius: BorderRadius.circular(16),
        ),
        child: Column(children: [
          Icon(icon, size: 48, color: selected ? Theme.of(context).colorScheme.primary : Colors.grey),
          const SizedBox(height: 12),
          Text(label, style: TextStyle(fontWeight: FontWeight.w600, color: selected ? Theme.of(context).colorScheme.primary : null)),
        ]),
      ),
    );
  }
}
