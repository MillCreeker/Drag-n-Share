import 'package:flutter/material.dart';

var appTheme = ThemeData(
    primaryColor: Colors.green[700],
    fontFamily: 'Nunito',
    brightness: Brightness.dark,
    bottomAppBarTheme: const BottomAppBarTheme(),
    textTheme: const TextTheme(
      bodyLarge: TextStyle(
        fontSize: 18,
        // color: Colors.teal,
        fontWeight: FontWeight.bold,
      ),
      bodyMedium: TextStyle(
        fontSize: 16,
        color: Colors.white70,
        fontWeight: FontWeight.w900,
      ),
      bodySmall: TextStyle(
        fontSize: 14,
      ),
      labelLarge: TextStyle(
        letterSpacing: 1.3,
        fontWeight: FontWeight.bold,
      ),
      labelMedium: TextStyle(),
      labelSmall: TextStyle(),
      displayLarge: TextStyle(
        fontWeight: FontWeight.bold,
      ),
      displayMedium: TextStyle(),
      displaySmall: TextStyle(),
      titleLarge: TextStyle(
        fontSize: 20,
        fontWeight: FontWeight.bold,
      ),
      titleMedium: TextStyle(
        fontSize: 18,
        fontWeight: FontWeight.bold,
        letterSpacing: 1.4,
        // color: Colors.teal,
      ),
      titleSmall: TextStyle(),
      headlineLarge: TextStyle(),
      headlineMedium: TextStyle(),
      headlineSmall: TextStyle(),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ButtonStyle(
        backgroundColor: MaterialStatePropertyAll(Colors.green[700]),
        overlayColor: MaterialStatePropertyAll(Colors.amber[700]),
        textStyle: const MaterialStatePropertyAll(TextStyle(
          fontWeight: FontWeight.bold,
        )),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: ButtonStyle(
        foregroundColor: MaterialStatePropertyAll(Colors.green[700]),
        overlayColor: MaterialStatePropertyAll(Colors.amber[700]),
        textStyle: const MaterialStatePropertyAll(TextStyle(
          fontSize: 16,
          fontWeight: FontWeight.bold,
        )),
      ),
    ),
    outlinedButtonTheme: const OutlinedButtonThemeData(
      style: ButtonStyle(),
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: MaterialStatePropertyAll(Colors.amber[700]),
      checkColor: const MaterialStatePropertyAll(Colors.black38),
    ),
    bottomNavigationBarTheme: BottomNavigationBarThemeData(
      selectedItemColor: Colors.green[700],
    ),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
        backgroundColor: Colors.green[700], splashColor: Colors.amber[700]));
