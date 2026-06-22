import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'providers/vault_provider.dart';
import 'providers/search_provider.dart';
import 'providers/sync_provider.dart';
import 'providers/settings_provider.dart';
import 'screens/home_screen.dart';
import 'screens/wizard_screen.dart';

const Color kStratumOrange = Color(0xFFFF2A00);

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setPreferredOrientations([
    DeviceOrientation.landscapeLeft,
    DeviceOrientation.landscapeRight,
    DeviceOrientation.portraitUp,
  ]);
  runApp(const StratumApp());
}

class StratumApp extends StatefulWidget {
  const StratumApp({super.key});

  @override
  State<StratumApp> createState() => _StratumAppState();
}

class _StratumAppState extends State<StratumApp> {
  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) {
          final s = SettingsProvider();
          s.loadSettings();
          return s;
        }),
        ChangeNotifierProvider(create: (_) => VaultProvider()),
        ChangeNotifierProvider(create: (_) => SearchProvider()),
        ChangeNotifierProvider(create: (_) => SyncProvider()),
      ],
      child: Consumer<SettingsProvider>(
        builder: (context, settings, _) {
          final isDark = settings.loaded ? settings.theme.darkMode : true;
          // Wire vault path to backend
          if (settings.loaded && settings.config.vaultPath.isNotEmpty) {
            final vault = context.read<VaultProvider>();
            if (vault.vaultPath != settings.config.vaultPath) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                vault.configureVault(settings.config.vaultPath);
              });
            }
          }
          return MaterialApp(
            title: 'Stratum',
            debugShowCheckedModeBanner: false,
            theme: ThemeData(
              useMaterial3: true,
              colorScheme: ColorScheme.fromSeed(
                seedColor: kStratumOrange,
                brightness: Brightness.light,
              ),
              fontFamily: 'Inter, sans-serif',
            ),
            darkTheme: ThemeData(
              useMaterial3: true,
              colorScheme: ColorScheme.fromSeed(
                seedColor: kStratumOrange,
                brightness: Brightness.dark,
              ),
              fontFamily: 'Inter, sans-serif',
            ),
            themeMode: isDark ? ThemeMode.dark : ThemeMode.light,
            home: settings.loaded && settings.isFirstLaunch
                ? const WizardScreen()
                : const HomeScreen(),
          );
        },
      ),
    );
  }
}
