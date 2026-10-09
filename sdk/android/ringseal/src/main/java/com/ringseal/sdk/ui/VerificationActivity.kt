package com.ringseal.sdk.ui

import android.content.Context
import android.content.Intent
import android.graphics.Typeface
import android.hardware.display.DisplayManager
import android.os.Bundle
import android.provider.Settings
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.view.WindowManager
import android.view.accessibility.AccessibilityManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.google.android.material.button.MaterialButton
import com.google.android.material.card.MaterialCardView
import com.ringseal.sdk.RingsealClient
import com.ringseal.sdk.RingsealConfig
import com.ringseal.sdk.SessionStatus
import com.ringseal.sdk.VerificationSession
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Locale
import java.util.TimeZone

class VerificationActivity : AppCompatActivity() {

    companion object {
        private const val EXTRA_CONFIG = "extra_config"
        private const val EXTRA_AUTH_TOKEN = "extra_auth_token"
        private const val EXTRA_THEME = "extra_theme"

        fun createIntent(
            context: Context,
            config: RingsealConfig,
            authToken: String,
            theme: RingsealTheme = RingsealTheme()
        ): Intent {
            return Intent(context, VerificationActivity::class.java).apply {
                putExtra(EXTRA_CONFIG, config)
                putExtra(EXTRA_AUTH_TOKEN, authToken)
                putExtra(EXTRA_THEME, theme)
            }
        }
    }

    private lateinit var config: RingsealConfig
    private lateinit var theme: RingsealTheme
    private lateinit var client: RingsealClient
    
    private var pollingJob: Job? = null
    private var countdownJob: Job? = null

    // UI Components
    private lateinit var rootLayout: FrameLayout
    private lateinit var loadingView: ProgressBar
    private lateinit var contentLayout: LinearLayout
    private lateinit var emptyStateView: TextView
    private lateinit var codeContainer: LinearLayout
    private lateinit var timerText: TextView
    private lateinit var digitViews: List<TextView>

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        @Suppress("DEPRECATION")
        config = intent.getSerializableExtra(EXTRA_CONFIG) as? RingsealConfig 
            ?: return finish()
            
        val authToken = intent.getStringExtra(EXTRA_AUTH_TOKEN) ?: return finish()
        
        @Suppress("DEPRECATION")
        theme = intent.getSerializableExtra(EXTRA_THEME) as? RingsealTheme ?: RingsealTheme()

        if (config.enableScreenshotProtection) {
            window.setFlags(
                WindowManager.LayoutParams.FLAG_SECURE,
                WindowManager.LayoutParams.FLAG_SECURE
            )
        }

        client = RingsealClient(config)
        client.setAuthToken(authToken)

        setupUI()
        checkSecurityThreats()
        startPolling()
    }

    private fun checkSecurityThreats() {
        // Overlay detection
        if (Settings.canDrawOverlays(this)) {
            // Log or warn about overlays
        }
        
        // Untrusted accessibility services
        val am = getSystemService(Context.ACCESSIBILITY_SERVICE) as AccessibilityManager
        val enabledServices = am.getEnabledAccessibilityServiceList(android.accessibilityservice.AccessibilityServiceInfo.FEEDBACK_ALL_MASK)
        if (enabledServices.isNotEmpty()) {
            // Log or warn about accessibility services
        }
        
        // External displays
        val dm = getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
        val displays = dm.displays
        if (displays.size > 1) {
            // Log or warn about screen mirroring
        }
    }

    private fun setupUI() {
        rootLayout = FrameLayout(this).apply {
            setBackgroundColor(theme.backgroundColor)
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
        }

        // Close Button
        val closeButton = MaterialButton(this, null, com.google.android.material.R.attr.materialIconButton).apply {
            text = "✕"
            setTextColor(theme.codeTextColor)
            textSize = 24f
            setOnClickListener { finish() }
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                setMargins(16.dpToPx(), 16.dpToPx(), 0, 0)
            }
        }
        rootLayout.addView(closeButton)

        // Loading View
        loadingView = ProgressBar(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { gravity = Gravity.CENTER }
        }
        rootLayout.addView(loadingView)

        // Empty State
        emptyStateView = TextView(this).apply {
            text = "No active verification"
            textSize = theme.titleTextSize
            setTextColor(theme.codeTextColor)
            gravity = Gravity.CENTER
            visibility = android.view.View.GONE
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { gravity = Gravity.CENTER }
        }
        rootLayout.addView(emptyStateView)

        // Content Layout (Title, Code, Timer, Warning)
        contentLayout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            visibility = android.view.View.GONE
            layoutParams = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { 
                gravity = Gravity.CENTER 
                setMargins(32.dpToPx(), 0, 32.dpToPx(), 0)
            }
        }

        val titleView = TextView(this).apply {
            text = "Verification Code"
            textSize = theme.titleTextSize
            setTextColor(theme.codeTextColor)
            typeface = Typeface.DEFAULT_BOLD
            gravity = Gravity.CENTER
        }
        contentLayout.addView(titleView, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ).apply { bottomMargin = 32.dpToPx() })

        // Code Container
        codeContainer = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
        }
        
        digitViews = (0 until 6).map {
            val card = MaterialCardView(this).apply {
                setCardBackgroundColor(theme.codeBoxColor)
                radius = 8.dpToPx().toFloat()
                cardElevation = 2.dpToPx().toFloat()
            }
            
            val tv = TextView(this).apply {
                textSize = theme.codeTextSize
                setTextColor(theme.codeTextColor)
                typeface = Typeface.MONOSPACE
                gravity = Gravity.CENTER
                setPadding(16.dpToPx(), 16.dpToPx(), 16.dpToPx(), 16.dpToPx())
                importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
            }
            
            card.addView(tv, FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { gravity = Gravity.CENTER })
            
            codeContainer.addView(card, LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(4.dpToPx(), 0, 4.dpToPx(), 0) })
            
            tv
        }
        
        contentLayout.addView(codeContainer, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ).apply { bottomMargin = 24.dpToPx() })

        timerText = TextView(this).apply {
            textSize = 16f
            setTextColor(theme.primaryColor)
            gravity = Gravity.CENTER
        }
        contentLayout.addView(timerText, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ).apply { bottomMargin = 32.dpToPx() })

        val warningView = TextView(this).apply {
            text = "Never read this code aloud. Your bank will never ask you to share your screen."
            textSize = 14f
            setTextColor(theme.warningColor)
            gravity = Gravity.CENTER
            typeface = Typeface.DEFAULT_BOLD
        }
        contentLayout.addView(warningView, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ))

        rootLayout.addView(contentLayout)
        setContentView(rootLayout)
    }

    private fun startPolling() {
        pollingJob?.cancel()
        pollingJob = lifecycleScope.launch {
            while (isActive) {
                try {
                    val session = client.getActiveSession()
                    updateState(session)
                } catch (e: Exception) {
                    // Log error in production
                    e.printStackTrace()
                }
                delay(config.pollingIntervalMs)
            }
        }
    }

    private fun updateState(session: VerificationSession?) {
        loadingView.visibility = android.view.View.GONE
        
        if (session == null || session.status != SessionStatus.ACTIVE) {
            contentLayout.visibility = android.view.View.GONE
            emptyStateView.visibility = android.view.View.VISIBLE
            countdownJob?.cancel()
            return
        }

        emptyStateView.visibility = android.view.View.GONE
        contentLayout.visibility = android.view.View.VISIBLE

        val code = session.code.padEnd(6, ' ')
        for (i in 0 until 6) {
            digitViews[i].text = code.getOrNull(i)?.toString() ?: ""
        }

        startCountdown(session.expiresAt)
    }

    private fun startCountdown(expiresAtIso: String) {
        countdownJob?.cancel()
        countdownJob = lifecycleScope.launch {
            val format = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US).apply {
                timeZone = TimeZone.getTimeZone("UTC")
            }
            
            try {
                val expiresAt = format.parse(expiresAtIso)?.time ?: return@launch
                
                while (isActive) {
                    val remainingMs = expiresAt - System.currentTimeMillis()
                    if (remainingMs <= 0) {
                        timerText.text = "Expired"
                        break
                    }
                    
                    val seconds = remainingMs / 1000
                    timerText.text = String.format(Locale.US, "Expires in %02d:%02d", seconds / 60, seconds % 60)
                    delay(1000)
                }
            } catch (e: Exception) {
                timerText.text = ""
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        pollingJob?.cancel()
        countdownJob?.cancel()
        client.shutdown()
    }

    private fun Int.dpToPx(): Int {
        return TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_DIP,
            this.toFloat(),
            resources.displayMetrics
        ).toInt()
    }
}
