import 'package:flutter/material.dart';
import 'package:drag_n_share/overview/overview_screen.dart';
import 'package:drag_n_share/splash/splash_screen.dart';

var appRoutes = <String, WidgetBuilder>{
  '/': (_) => const OverviewScreen(),
  '/loading': (_) => const SplashScreen(),
};