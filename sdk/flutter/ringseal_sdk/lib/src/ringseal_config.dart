/// Configuration options for the Ringseal SDK.
class RingsealConfig {
  /// The base URL of the Ringseal backend API.
  final String baseUrl;
  
  /// How often to poll the backend for active session status.
  final Duration pollingInterval;
  
  /// The expected lifetime of a verification code (for UI countdown).
  final Duration codeExpiry;
  
  /// Whether to enable platform-specific screenshot protection.
  final bool enableScreenshotProtection;

  /// Whether to allow HTTP (for development).
  final bool allowInsecure;
  
  /// Creates a new configuration instance.
  RingsealConfig({
    required this.baseUrl,
    this.pollingInterval = const Duration(seconds: 3),
    this.codeExpiry = const Duration(seconds: 60),
    this.enableScreenshotProtection = true,
    this.allowInsecure = false,
  }) {
    assert(allowInsecure || baseUrl.startsWith('https://'), 'baseUrl must use HTTPS');
  }
}
