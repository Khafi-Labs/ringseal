# Ringseal Flutter SDK

Embed secure call verification in your banking app with Ringseal.

## Installation

Add this to your package's `pubspec.yaml` file:

```yaml
dependencies:
  ringseal_sdk: ^0.1.0
```

## Quick Start

1. Initialize the client:
```dart
final client = RingsealClient(
  config: RingsealConfig(
    baseUrl: 'https://api.yourbank.com/ringseal',
  ),
);
```

2. Set the auth token after user logs in:
```dart
client.setAuthToken('your-jwt-token-here');
```

3. Display the verification screen widget:
```dart
RingsealVerificationScreen(
  client: client,
  onSessionVerified: () {
    print("Session verified!");
  },
);
```

## Configuration

You can customize the SDK's behavior using `RingsealConfig`:
- `baseUrl`: The URL of your API gateway routing to Ringseal.
- `pollingInterval`: How often to check for session status (default 3s).
- `codeExpiry`: Verification code visual expiry duration.
- `enableScreenshotProtection`: (Placeholder) prevent capturing the verification code.

## Theming

Use `RingsealTheme` to match your bank's branding:
```dart
RingsealTheme(
  primaryColor: Colors.teal,
  codeBoxColor: Colors.grey[200],
  codeTextColor: Colors.black,
  warningColor: Colors.red,
  codeBoxRadius: BorderRadius.circular(10),
)
```

## API Reference

The SDK interacts with the `GET /api/v1/verify` endpoint via HTTP using the JWT provided to `setAuthToken`. Standard error handling and timeout features are implemented in the API client.
