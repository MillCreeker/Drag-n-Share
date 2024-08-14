import 'package:flutter/material.dart';

class OverviewScreen extends StatelessWidget {
  const OverviewScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Drag-n-Share'),
      ),
      body: const Center(
        child: Text('Overview Screen'),
      ),
    );
  }
}