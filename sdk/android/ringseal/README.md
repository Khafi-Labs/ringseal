# Ringseal Android SDK

Ringseal is a caller-verification API for banks. The SDK is embedded in an Android app to display verification codes to customers.

## Installation

Add the dependency to your app's `build.gradle.kts`:

```kotlin
dependencies {
    implementation(project(":ringseal"))
    // or if published to Maven:
    // implementation("com.ringseal.sdk:ringseal:1.0.0")
}
```

## Usage

Create a configuration and launch the verification activity with a user's token.

```kotlin
import com.ringseal.sdk.RingsealConfig
import com.ringseal.sdk.ui.VerificationActivity
import com.ringseal.sdk.ui.RingsealTheme

// 1. Configure the SDK
val config = RingsealConfig(
    baseUrl = "https://api.ringseal.com",
    enableScreenshotProtection = true
)

// 2. Launch the Activity
val intent = VerificationActivity.createIntent(
    context = this,
    config = config,
    authToken = "YOUR_BEARER_TOKEN",
    theme = RingsealTheme(
        primaryColor = 0xFF1A73E8.toInt()
    )
)
startActivity(intent)
```

## Theming

You can customize the appearance of the verification screen by passing a `RingsealTheme` object:

```kotlin
val theme = RingsealTheme(
    primaryColor = 0xFF1A73E8.toInt(),
    backgroundColor = 0xFFF8F9FA.toInt(),
    codeBoxColor = 0xFFFFFFFF.toInt(),
    codeTextColor = 0xFF202124.toInt(),
    warningColor = 0xFFEA4335.toInt()
)
```

## Security Features

By default, screenshot protection is enabled via `FLAG_SECURE`. You can disable it for testing:
```kotlin
val config = RingsealConfig(
    baseUrl = "https://api.ringseal.com",
    enableScreenshotProtection = false
)
```
