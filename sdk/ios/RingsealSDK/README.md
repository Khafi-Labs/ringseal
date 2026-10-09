# RingsealSDK

Ringseal SDK for iOS provides a secure way to embed caller verification codes in your banking application.

## Installation

### Swift Package Manager
The package lives in this monorepo at `sdk/ios/RingsealSDK`. Clone the repository and add it in Xcode with **File > Add Package Dependencies > Add Local...**, or vendor the folder into your project.

## Quick Start

### Configuration
First, initialize the `RingsealClient` with your API URL and set the user's auth token.

```swift
import RingsealSDK

let url = URL(string: "https://api.yourbank.com/ringseal")!
let config = RingsealConfig(baseURL: url)
let client = RingsealClient(config: config)

// After user login:
client.setAuthToken("your_bearer_token")
```

### SwiftUI
Embed the `RingsealVerificationView` anywhere in your SwiftUI hierarchy:

```swift
import SwiftUI
import RingsealSDK

struct VerificationScreen: View {
    let client: RingsealClient
    
    var body: some View {
        RingsealVerificationView(client: client)
            .navigationTitle("Caller Verification")
    }
}
```

### UIKit
Present or push the `RingsealVerificationViewController`:

```swift
import UIKit
import RingsealSDK

class MainViewController: UIViewController {
    let client: RingsealClient // initialized previously
    
    func showVerification() {
        let vc = RingsealVerificationViewController()
        vc.client = client
        vc.delegate = self
        navigationController?.pushViewController(vc, animated: true)
    }
}

extension MainViewController: RingsealVerificationDelegate {
    func ringsealDidFindActiveSession(_ session: VerificationSession) {
        print("Active session found: \\(session.code)")
    }
    
    func ringsealDidFindNoActiveSession() {
        print("No active session.")
    }
    
    func ringsealDidEncounterError(_ error: Error) {
        print("Error checking session: \\(error)")
    }
}
```

## Customization & Theming
You can customize the appearance by providing a custom `RingsealTheme`:

```swift
var customTheme = RingsealTheme.default
customTheme.primaryColor = .blue
customTheme.codeBoxColor = .lightGray

let vc = RingsealVerificationViewController()
vc.theme = customTheme
```

## Security
Screenshot protection is enabled by default. It obscures the UI when the app goes into the background and shows alerts when the user attempts to screenshot the verification code.
