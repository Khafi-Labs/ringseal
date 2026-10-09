import UIKit

/// Defines the appearance of the Ringseal UI components.
public struct RingsealTheme {
    public var primaryColor: UIColor
    public var backgroundColor: UIColor
    public var codeBoxColor: UIColor
    public var codeTextColor: UIColor
    public var warningColor: UIColor
    public var titleFont: UIFont
    public var codeFont: UIFont
    public var subtitleFont: UIFont
    public var codeBoxCornerRadius: CGFloat
    
    /// The default theme configuration.
    public static let `default` = RingsealTheme(
        primaryColor: .label,
        backgroundColor: .systemBackground,
        codeBoxColor: .secondarySystemBackground,
        codeTextColor: .label,
        warningColor: .systemRed,
        titleFont: .systemFont(ofSize: 24, weight: .bold),
        codeFont: .monospacedDigitSystemFont(ofSize: 32, weight: .bold),
        subtitleFont: .systemFont(ofSize: 15, weight: .medium),
        codeBoxCornerRadius: 8.0
    )
    
    public init(
        primaryColor: UIColor,
        backgroundColor: UIColor,
        codeBoxColor: UIColor,
        codeTextColor: UIColor,
        warningColor: UIColor,
        titleFont: UIFont,
        codeFont: UIFont,
        subtitleFont: UIFont,
        codeBoxCornerRadius: CGFloat
    ) {
        self.primaryColor = primaryColor
        self.backgroundColor = backgroundColor
        self.codeBoxColor = codeBoxColor
        self.codeTextColor = codeTextColor
        self.warningColor = warningColor
        self.titleFont = titleFont
        self.codeFont = codeFont
        self.subtitleFont = subtitleFont
        self.codeBoxCornerRadius = codeBoxCornerRadius
    }
}
