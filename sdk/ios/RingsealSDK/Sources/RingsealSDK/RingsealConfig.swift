import Foundation

/// Configuration for the Ringseal SDK.
public struct RingsealConfig: Sendable {
    /// The base URL of the Ringseal API.
    public let baseURL: URL
    
    /// The interval in seconds to poll for active sessions.
    public let pollingInterval: TimeInterval
    
    /// The duration in seconds that a code remains valid.
    public let codeExpiry: TimeInterval
    
    /// Whether to enable screenshot protection for the verification code.
    public let enableScreenshotProtection: Bool

    /// Whether to allow insecure HTTP connections (for testing).
    public let allowInsecure: Bool
    
    /// Initializes a new configuration.
    /// - Parameters:
    ///   - baseURL: The base URL of the Ringseal API.
    ///   - pollingInterval: The interval in seconds to poll for active sessions. Defaults to 3.0.
    ///   - codeExpiry: The duration in seconds that a code remains valid. Defaults to 60.0.
    ///   - enableScreenshotProtection: Whether to enable screenshot protection. Defaults to true.
    ///   - allowInsecure: Whether to allow HTTP. Defaults to false.
    public init(baseURL: URL, pollingInterval: TimeInterval = 3.0, codeExpiry: TimeInterval = 60.0, enableScreenshotProtection: Bool = true, allowInsecure: Bool = false) {
        precondition(allowInsecure || baseURL.scheme == "https", "baseURL must use HTTPS in production.")
        self.baseURL = baseURL
        self.pollingInterval = pollingInterval
        self.codeExpiry = codeExpiry
        self.enableScreenshotProtection = enableScreenshotProtection
        self.allowInsecure = allowInsecure
    }
}
