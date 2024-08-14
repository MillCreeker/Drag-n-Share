import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:localization/localization.dart';
import 'package:drag_n_share/theme.dart';
import 'package:drag_n_share/routes.dart';

Future<void> main() async {
  runApp(const Main());
}

class Main extends StatelessWidget {
  const Main({super.key});

  @override
  Widget build(BuildContext context) {
    LocalJsonLocalization.delegate.directories = ['lib/i18n'];

    return MaterialApp(
      localizationsDelegates: [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        LocalJsonLocalization.delegate,
      ],
      supportedLocales: const [
        Locale('en', 'US'),
        Locale('de', 'DE'),
      ],
      localeResolutionCallback: (locale, supportedLocales) {
        if (supportedLocales.contains(locale)) {
          return locale;
        }

        if (locale?.languageCode == 'de') {
          return const Locale('de', 'DE');
        }

        return const Locale('en', 'US');
      },
      title: 'Drag-n-Share',
      theme: appTheme,
      routes: appRoutes,
      initialRoute: '/loading',
    );
  }
}

class FlutterNativeSplash {
}
