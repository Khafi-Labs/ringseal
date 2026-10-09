import Foundation

/// Represents the status of a verification session.
public enum SessionStatus: String, Codable, Sendable {
    case created
    case active
    case verified
    case expired
    case ended
}

/// A verification session containing the code to display.
public struct VerificationSession: Codable, Sendable {
    /// The unique identifier of the session.
    public let sessionId: String
    
    /// The verification code.
    public let code: String
    
    /// The time at which the session and code expire.
    public let expiresAt: Date
    
    /// The current status of the session.
    public let status: SessionStatus
    
    enum CodingKeys: String, CodingKey {
        case sessionId = "session_id"
        case code
        case expiresAt = "expires_at"
        case status
    }
}

/// An error returned by the Ringseal API.
public struct RingsealError: Error, Codable, Sendable {
    public let code: String
    public let message: String
}

/// Errors that can occur within the Ringseal SDK.
public enum RingsealSDKError: Error, LocalizedError {
    /// Authentication token has not been set.
    case notAuthenticated
    
    /// A network error occurred.
    case networkError(Error)
    
    /// The server returned an error response.
    case serverError(RingsealError)
    
    /// Failed to decode the response.
    case decodingError(Error)
    
    /// The constructed URL is invalid.
    case invalidURL
    
    public var errorDescription: String? {
        switch self {
        case .notAuthenticated:
            return "Auth token is not set. Call setAuthToken(_:) before using the client."
        case .networkError(let error):
            return "Network error: \(error.localizedDescription)"
        case .serverError(let error):
            return "Server error: \(error.message) (Code: \(error.code))"
        case .decodingError(let error):
            return "Failed to decode response: \(error.localizedDescription)"
        case .invalidURL:
            return "The API URL is invalid."
        }
    }
}
