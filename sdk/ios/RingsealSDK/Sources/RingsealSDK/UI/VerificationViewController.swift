import UIKit

/// Delegate protocol for responding to events from the verification view controller.
public protocol RingsealVerificationDelegate: AnyObject {
    func ringsealDidFindActiveSession(_ session: VerificationSession)
    func ringsealDidFindNoActiveSession()
    func ringsealDidEncounterError(_ error: Error)
}

/// A view controller that displays the verification code and polls for active sessions.
public class RingsealVerificationViewController: UIViewController {
    public var client: RingsealClient!
    public weak var delegate: RingsealVerificationDelegate?
    public var theme: RingsealTheme = .default
    
    private var pollingTask: Task<Void, Never>?
    private var timer: Timer?
    private var currentSession: VerificationSession?
    
    // UI Elements
    private let loadingSpinner = UIActivityIndicatorView(style: .large)
    private let titleLabel = UILabel()
    private let statusLabel = UILabel()
    private let countdownLabel = UILabel()
    private let warningLabel = UILabel()
    private let codeStackView = UIStackView()
    private var digitLabels: [UILabel] = []
    
    private let noSessionContainer = UIView()
    private let noSessionIcon = UIImageView()
    private let noSessionLabel = UILabel()
    
    // Screenshot Protection
    private let overlayView = UIVisualEffectView(effect: UIBlurEffect(style: .dark))

    public override func viewDidLoad() {
        super.viewDidLoad()
        setupUI()
        setupScreenshotProtection()
        
        NotificationCenter.default.addObserver(self, selector: #selector(screenCaptureChanged), name: UIScreen.capturedDidChangeNotification, object: nil)
        screenCaptureChanged()
    }
    
    @objc private func screenCaptureChanged() {
        if UIScreen.main.isCaptured {
            codeStackView.isHidden = true
            warningLabel.text = "Screen recording detected. Code is hidden for your security."
            warningLabel.textColor = .red
        } else {
            codeStackView.isHidden = false
            warningLabel.text = "Never read this code aloud. Your bank will never ask you to share your screen."
            warningLabel.textColor = theme.warningColor
        }
    }
    
    public override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        startPolling()
    }
    
    public override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        stopPolling()
        timer?.invalidate()
    }
    
    private func setupUI() {
        view.backgroundColor = theme.backgroundColor
        
        // Title
        titleLabel.text = "Caller Verification"
        titleLabel.font = theme.titleFont
        titleLabel.textColor = theme.primaryColor
        titleLabel.textAlignment = .center
        
        // Code Stack
        codeStackView.axis = .horizontal
        codeStackView.spacing = 8
        codeStackView.distribution = .fillEqually
        
        for _ in 0..<6 {
            let container = UIView()
            container.backgroundColor = theme.codeBoxColor
            container.layer.cornerRadius = theme.codeBoxCornerRadius
            container.layer.masksToBounds = true
            
            let label = UILabel()
            label.font = theme.codeFont
            label.textColor = theme.codeTextColor
            label.textAlignment = .center
            label.text = "-"
            label.isAccessibilityElement = false // D-01 Prevent accessibility reads
            digitLabels.append(label)
            
            container.addSubview(label)
            label.translatesAutoresizingMaskIntoConstraints = false
            NSLayoutConstraint.activate([
                label.centerXAnchor.constraint(equalTo: container.centerXAnchor),
                label.centerYAnchor.constraint(equalTo: container.centerYAnchor)
            ])
            
            container.translatesAutoresizingMaskIntoConstraints = false
            container.widthAnchor.constraint(equalToConstant: 44).isActive = true
            container.heightAnchor.constraint(equalToConstant: 56).isActive = true
            
            codeStackView.addArrangedSubview(container)
        }
        
        // Status & Countdown
        statusLabel.font = theme.subtitleFont
        statusLabel.textColor = .secondaryLabel
        statusLabel.textAlignment = .center
        statusLabel.text = "Checking for active sessions..."
        
        countdownLabel.font = theme.subtitleFont
        countdownLabel.textColor = .secondaryLabel
        countdownLabel.textAlignment = .center
        
        // Warning
        warningLabel.text = "Never read this code aloud. Your bank will never ask you to share your screen."
        warningLabel.font = theme.subtitleFont
        warningLabel.textColor = theme.warningColor
        warningLabel.textAlignment = .center
        warningLabel.numberOfLines = 0
        
        // No Session View
        noSessionContainer.isHidden = true
        noSessionIcon.image = UIImage(systemName: "shield.slash")
        noSessionIcon.tintColor = .secondaryLabel
        noSessionIcon.contentMode = .scaleAspectFit
        
        noSessionLabel.text = "No Active Session"
        noSessionLabel.font = theme.titleFont
        noSessionLabel.textColor = .secondaryLabel
        noSessionLabel.textAlignment = .center
        
        noSessionContainer.addSubview(noSessionIcon)
        noSessionContainer.addSubview(noSessionLabel)
        
        // Layout
        let mainStack = UIStackView(arrangedSubviews: [
            titleLabel, loadingSpinner, statusLabel, codeStackView, countdownLabel, warningLabel, noSessionContainer
        ])
        mainStack.axis = .vertical
        mainStack.spacing = 20
        mainStack.alignment = .center
        
        view.addSubview(mainStack)
        mainStack.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            mainStack.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            mainStack.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            mainStack.leadingAnchor.constraint(greaterThanOrEqualTo: view.leadingAnchor, constant: 20),
            mainStack.trailingAnchor.constraint(lessThanOrEqualTo: view.trailingAnchor, constant: -20)
        ])
        
        noSessionIcon.translatesAutoresizingMaskIntoConstraints = false
        noSessionLabel.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            noSessionIcon.centerXAnchor.constraint(equalTo: noSessionContainer.centerXAnchor),
            noSessionIcon.topAnchor.constraint(equalTo: noSessionContainer.topAnchor),
            noSessionIcon.widthAnchor.constraint(equalToConstant: 60),
            noSessionIcon.heightAnchor.constraint(equalToConstant: 60),
            noSessionLabel.topAnchor.constraint(equalTo: noSessionIcon.bottomAnchor, constant: 16),
            noSessionLabel.leadingAnchor.constraint(equalTo: noSessionContainer.leadingAnchor),
            noSessionLabel.trailingAnchor.constraint(equalTo: noSessionContainer.trailingAnchor),
            noSessionLabel.bottomAnchor.constraint(equalTo: noSessionContainer.bottomAnchor)
        ])
        
        loadingSpinner.startAnimating()
        showLoadingState()
        
        overlayView.frame = view.bounds
        overlayView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        overlayView.isHidden = true
        view.addSubview(overlayView)
    }
    
    private func showLoadingState() {
        loadingSpinner.isHidden = false
        statusLabel.isHidden = false
        statusLabel.text = "Checking for active sessions..."
        codeStackView.isHidden = true
        countdownLabel.isHidden = true
        warningLabel.isHidden = true
        noSessionContainer.isHidden = true
    }
    
    private func showActiveSession(_ session: VerificationSession) {
        loadingSpinner.isHidden = true
        statusLabel.isHidden = false
        statusLabel.text = "Your verification code is:"
        
        if UIScreen.main.isCaptured {
            codeStackView.isHidden = true
        } else {
            codeStackView.isHidden = false
        }
        
        countdownLabel.isHidden = false
        warningLabel.isHidden = false
        noSessionContainer.isHidden = true
        
        let chars = Array(session.code)
        for (index, label) in digitLabels.enumerated() {
            if index < chars.count {
                label.text = String(chars[index])
            } else {
                label.text = "-"
            }
        }
        
        startCountdown(to: session.expiresAt)
    }
    
    private func showNoSession() {
        loadingSpinner.isHidden = true
        statusLabel.isHidden = true
        codeStackView.isHidden = true
        countdownLabel.isHidden = true
        warningLabel.isHidden = true
        noSessionContainer.isHidden = false
    }
    
    private func startPolling() {
        pollingTask = Task {
            while !Task.isCancelled {
                do {
                    if let session = try await client.getActiveSession() {
                        if session.status == .active {
                            self.currentSession = session
                            await MainActor.run {
                                self.showActiveSession(session)
                                self.delegate?.ringsealDidFindActiveSession(session)
                            }
                        } else {
                            await MainActor.run {
                                self.showNoSession()
                            }
                        }
                    } else {
                        await MainActor.run {
                            self.showNoSession()
                            self.delegate?.ringsealDidFindNoActiveSession()
                        }
                    }
                } catch {
                    await MainActor.run {
                        self.delegate?.ringsealDidEncounterError(error)
                    }
                }
                
                // Wait for polling interval (default 3 seconds)
                try? await Task.sleep(nanoseconds: 3_000_000_000)
            }
        }
    }
    
    private func stopPolling() {
        pollingTask?.cancel()
        pollingTask = nil
    }
    
    private func startCountdown(to expiry: Date) {
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            let remaining = expiry.timeIntervalSinceNow
            if remaining <= 0 {
                self?.countdownLabel.text = "Code expired"
                self?.timer?.invalidate()
                // Force a poll
                self?.stopPolling()
                self?.startPolling()
            } else {
                let minutes = Int(remaining) / 60
                let seconds = Int(remaining) % 60
                self?.countdownLabel.text = String(format: "Expires in %d:%02d", minutes, seconds)
            }
        }
    }
    
    private func setupScreenshotProtection() {
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(appDidBecomeActive),
            name: UIApplication.didBecomeActiveNotification,
            object: nil
        )
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(appWillResignActive),
            name: UIApplication.willResignActiveNotification,
            object: nil
        )
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(userDidTakeScreenshot),
            name: UIApplication.userDidTakeScreenshotNotification,
            object: nil
        )
    }
    
    @objc private func appDidBecomeActive() {
        overlayView.isHidden = true
    }
    
    @objc private func appWillResignActive() {
        overlayView.isHidden = false
    }
    
    @objc private func userDidTakeScreenshot() {
        // Optional: show a warning alert if they took a screenshot
        let alert = UIAlertController(title: "Security Warning", message: "Screenshots are discouraged for security reasons. Please do not share this code.", preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "OK", style: .default))
        present(alert, animated: true)
    }
}
