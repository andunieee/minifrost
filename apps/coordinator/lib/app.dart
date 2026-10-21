import 'package:flutter/material.dart';

import 'pages/settings.dart';
import 'pages/signers.dart';
import 'pages/wallets.dart';
import 'src/rust/api.dart';

class CoordinatorRoot extends StatelessWidget {
  const CoordinatorRoot({super.key, required this.app});
  final CoordinatorApp app;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Minifrost Coordinator',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true),
      darkTheme: ThemeData(colorSchemeSeed: Colors.indigo, brightness: Brightness.dark, useMaterial3: true),
      home: Shell(app: app),
    );
  }
}

class Shell extends StatefulWidget {
  const Shell({super.key, required this.app});
  final CoordinatorApp app;
  @override
  State<Shell> createState() => _ShellState();
}

class _ShellState extends State<Shell> {
  int index = 0;

  @override
  Widget build(BuildContext context) {
    final pages = [
      WalletsPage(app: widget.app),
      SignersPage(app: widget.app),
      SettingsPage(app: widget.app),
    ];
    return Scaffold(
      body: Row(children: [
        NavigationRail(
          selectedIndex: index,
          onDestinationSelected: (i) => setState(() => index = i),
          labelType: NavigationRailLabelType.all,
          destinations: const [
            NavigationRailDestination(icon: Icon(Icons.account_balance_wallet_outlined), label: Text('Wallets')),
            NavigationRailDestination(icon: Icon(Icons.shield_outlined), label: Text('Signers')),
            NavigationRailDestination(icon: Icon(Icons.settings_outlined), label: Text('Settings')),
          ],
        ),
        const VerticalDivider(width: 1),
        // keyed so a tab remembers nothing stale after switching
        Expanded(child: KeyedSubtree(key: ValueKey(index), child: pages[index])),
      ]),
    );
  }
}
