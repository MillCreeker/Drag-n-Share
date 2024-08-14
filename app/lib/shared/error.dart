import 'package:flutter/material.dart';
import 'package:localization/localization.dart';

class ErrorScreen extends StatelessWidget {
  final String errorMessage;

  const ErrorScreen({
    super.key,
    this.errorMessage = '',
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        children: [
          const Spacer(),
          Icon(
            Icons.warning_rounded,
            size: 200,
            color: Colors.red[300],
          ),
          Text(
            errorMessage == ''
                ? 'unexpectedError'.i18n()
                : errorMessage,
            textAlign: TextAlign.center,
          ),
          const Spacer(),
        ],
      ),
    );
  }
}