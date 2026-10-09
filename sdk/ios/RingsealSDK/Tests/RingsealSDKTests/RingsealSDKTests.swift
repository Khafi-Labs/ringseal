import XCTest
@testable import RingsealSDK

final class RingsealSDKTests: XCTestCase {
    
    func testConfigInitialization() {
        let url = URL(string: "https://api.ringseal.test")!
        let config = RingsealConfig(baseURL: url)
        
        XCTAssertEqual(config.baseURL, url)
        XCTAssertEqual(config.pollingInterval, 3.0)
        XCTAssertEqual(config.codeExpiry, 60.0)
        XCTAssertTrue(config.enableScreenshotProtection)
    }
    
    func testSessionDecoding() throws {
        let json = """
        {
            "session_id": "123e4567-e89b-12d3-a456-426614174000",
            "code": "123456",
            "expires_at": "2026-10-04T18:56:39Z",
            "status": "active"
        }
        """.data(using: .utf8)!
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        
        let session = try decoder.decode(VerificationSession.self, from: json)
        
        XCTAssertEqual(session.sessionId, "123e4567-e89b-12d3-a456-426614174000")
        XCTAssertEqual(session.code, "123456")
        XCTAssertEqual(session.status, .active)
    }
    
    func testErrorDescriptions() {
        let error = RingsealSDKError.notAuthenticated
        XCTAssertEqual(error.localizedDescription, "Auth token is not set. Call setAuthToken(_:) before using the client.")
        
        let invalidUrl = RingsealSDKError.invalidURL
        XCTAssertEqual(invalidUrl.localizedDescription, "The API URL is invalid.")
    }
}
