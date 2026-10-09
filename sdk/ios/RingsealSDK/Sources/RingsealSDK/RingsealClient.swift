import Foundation

/// A client for communicating with the Ringseal API.
public final class RingsealClient: NSObject, Sendable, URLSessionDelegate {
    private let config: RingsealConfig
    private let session: URLSession
    
    // Use an actor or thread-safe storage for mutable state in Sendable types.
    // For simplicity here, we use a simple lock wrapper or actor.
    private actor TokenStorage {
        var token: String?
        func set(_ token: String) { self.token = token }
        func get() -> String? { return token }
    }
    private let tokenStorage = TokenStorage()
    
    /// Initializes a new Ringseal client.
    /// - Parameter config: The configuration to use.
    public init(config: RingsealConfig) {
        self.config = config
        
        let sessionConfig = URLSessionConfiguration.default
        sessionConfig.tlsMinimumSupportedProtocolVersion = .TLSv12
        
        super.init()
        self.session = URLSession(configuration: sessionConfig, delegate: self, delegateQueue: nil)
    }

    public func urlSession(_ session: URLSession, didReceive challenge: URLAuthenticationChallenge, completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        // TODO: Implement SSL pinning logic here with placeholder pins
        // For example, checking the server trust and comparing certificates against known pinned hashes.
        completionHandler(.performDefaultHandling, nil)
    }
    
    /// Sets the authentication token to use for API requests.
    /// - Parameter token: The Bearer token.
    public func setAuthToken(_ token: String) {
        Task {
            await tokenStorage.set(token)
        }
    }
    
    /// Fetches the active verification session, if any.
    /// - Returns: The active session, or nil if none exists (404).
    /// - Throws: `RingsealSDKError` if the request fails.
    public func getActiveSession() async throws -> VerificationSession? {
        guard let token = await tokenStorage.get() else {
            throw RingsealSDKError.notAuthenticated
        }
        
        guard let url = URL(string: "/api/v1/verify", relativeTo: config.baseURL) else {
            throw RingsealSDKError.invalidURL
        }
        
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        
        let data: Data
        let response: URLResponse
        
        do {
            (data, response) = try await session.data(for: request)
        } catch {
            throw RingsealSDKError.networkError(error)
        }
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw RingsealSDKError.networkError(URLError(.badServerResponse))
        }
        
        if httpResponse.statusCode == 404 {
            return nil
        }
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        
        guard (200...299).contains(httpResponse.statusCode) else {
            if let serverError = try? decoder.decode(RingsealError.self, from: data) {
                throw RingsealSDKError.serverError(serverError)
            } else {
                throw RingsealSDKError.networkError(URLError(.badServerResponse))
            }
        }
        
        do {
            let session = try decoder.decode(VerificationSession.self, from: data)
            return session
        } catch {
            throw RingsealSDKError.decodingError(error)
        }
    }
}
