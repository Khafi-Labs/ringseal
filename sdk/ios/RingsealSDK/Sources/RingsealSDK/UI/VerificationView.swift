import SwiftUI
import UIKit

@available(iOS 15.0, *)
/// A SwiftUI view that wraps `RingsealVerificationViewController`.
public struct RingsealVerificationView: UIViewControllerRepresentable {
    public let client: RingsealClient
    public var theme: RingsealTheme
    public var onActiveSession: ((VerificationSession) -> Void)?
    public var onNoActiveSession: (() -> Void)?
    public var onError: ((Error) -> Void)?
    
    public init(
        client: RingsealClient,
        theme: RingsealTheme = .default,
        onActiveSession: ((VerificationSession) -> Void)? = nil,
        onNoActiveSession: (() -> Void)? = nil,
        onError: ((Error) -> Void)? = nil
    ) {
        self.client = client
        self.theme = theme
        self.onActiveSession = onActiveSession
        self.onNoActiveSession = onNoActiveSession
        self.onError = onError
    }
    
    public func makeUIViewController(context: Context) -> RingsealVerificationViewController {
        let vc = RingsealVerificationViewController()
        vc.client = client
        vc.theme = theme
        vc.delegate = context.coordinator
        return vc
    }
    
    public func updateUIViewController(_ uiViewController: RingsealVerificationViewController, context: Context) {
        uiViewController.theme = theme
    }
    
    public func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    public class Coordinator: NSObject, RingsealVerificationDelegate {
        var parent: RingsealVerificationView
        
        init(_ parent: RingsealVerificationView) {
            self.parent = parent
        }
        
        public func ringsealDidFindActiveSession(_ session: VerificationSession) {
            parent.onActiveSession?(session)
        }
        
        public func ringsealDidFindNoActiveSession() {
            parent.onNoActiveSession?()
        }
        
        public func ringsealDidEncounterError(_ error: Error) {
            parent.onError?(error)
        }
    }
}
