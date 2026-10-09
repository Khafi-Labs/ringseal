import 'dart:convert';
import 'package:http/http.dart' as http;
import 'ringseal_config.dart';
import 'models.dart';

/// The primary API client for interacting with the Ringseal backend.
class RingsealClient {
  final RingsealConfig config;
  final http.Client _httpClient;
  String? _authToken;

  RingsealClient({
    required this.config,
    http.Client? httpClient,
  }) : _httpClient = httpClient ?? http.Client();

  /// Sets the authentication token to be used for API requests.
  /// The bank app should set this after the user logs in.
  void setAuthToken(String token) {
    _authToken = token;
  }

  /// Retrieves the active verification session for the current user.
  /// Returns null if there is no active session (404).
  /// Throws [RingsealError] if the API returns an error or fails.
  Future<VerificationSession?> getActiveSession() async {
    if (_authToken == null) {
      throw RingsealError(
        code: 'unauthorized',
        message: 'No auth token provided. Call setAuthToken() first.',
      );
    }

    try {
      final uri = Uri.parse('${config.baseUrl}/api/v1/verify');
      
      // TODO: Implement certificate pinning logic via platform channels or a specialized package
      
      final response = await _httpClient.get(
        uri,
        headers: {
          'Authorization': 'Bearer $_authToken',
          'Accept': 'application/json',
        },
      ).timeout(const Duration(seconds: 10));

      if (response.statusCode == 200) {
        final data = jsonDecode(response.body) as Map<String, dynamic>;
        return VerificationSession.fromJson(data);
      } else if (response.statusCode == 404) {
        // No active session
        return null;
      } else {
        String code = 'api_error';
        String message = 'API returned ${response.statusCode}';
        try {
          final errorData = jsonDecode(response.body) as Map<String, dynamic>;
          if (errorData.containsKey('code')) code = errorData['code'];
          if (errorData.containsKey('message')) message = errorData['message'];
        } catch (_) {}
        throw RingsealError(code: code, message: message);
      }
    } catch (e) {
      if (e is RingsealError) rethrow;
      throw RingsealError(
        code: 'network_error',
        message: 'Failed to contact Ringseal backend: ${e.toString()}',
      );
    }
  }

  /// Disposes resources held by the client.
  void dispose() {
    _httpClient.close();
  }
}
