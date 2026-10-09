import 'package:flutter/material.dart';

/// Theme configuration for the Ringseal verification screen.
class RingsealTheme {
  final Color primaryColor;
  final Color backgroundColor;
  final Color codeBoxColor;
  final Color codeTextColor;
  final Color warningColor;
  final TextStyle? titleStyle;
  final TextStyle? codeStyle;
  final TextStyle? subtitleStyle;
  final BorderRadius? codeBoxRadius;

  const RingsealTheme({
    this.primaryColor = Colors.blue,
    this.backgroundColor = Colors.white,
    this.codeBoxColor = const Color(0xFFF0F0F0),
    this.codeTextColor = Colors.black87,
    this.warningColor = Colors.orange,
    this.titleStyle,
    this.codeStyle,
    this.subtitleStyle,
    this.codeBoxRadius,
  });

  /// The default theme for the Ringseal SDK.
  static const RingsealTheme defaultTheme = RingsealTheme();
}
