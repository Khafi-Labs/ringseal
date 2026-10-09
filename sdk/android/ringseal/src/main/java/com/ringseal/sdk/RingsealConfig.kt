package com.ringseal.sdk

import java.io.Serializable

/**
 * Configuration for the Ringseal SDK.
 *
 * @property baseUrl The base URL of the Ringseal API (e.g., https://api.ringseal.com).
 * @property pollingIntervalMs Interval in milliseconds between polling requests. Default: 3000ms.
 * @property codeExpirySeconds The expected expiry duration of a code in seconds. Default: 60s.
 * @property enableScreenshotProtection Whether to enable FLAG_SECURE on the verification activity. Default: true.
 * @property connectTimeoutMs Connection timeout in milliseconds. Default: 10000ms.
 * @property readTimeoutMs Read timeout in milliseconds. Default: 10000ms.
 * @property allowInsecure Allow HTTP in development. Default: false.
 */
data class RingsealConfig(
    val baseUrl: String,
    val pollingIntervalMs: Long = 3000,
    val codeExpirySeconds: Int = 60,
    val enableScreenshotProtection: Boolean = true,
    val connectTimeoutMs: Long = 10000,
    val readTimeoutMs: Long = 10000,
    val allowInsecure: Boolean = false
) : Serializable {
    init {
        require(allowInsecure || baseUrl.startsWith("https://")) {
            "baseUrl must use HTTPS in production. Use allowInsecure=true only for testing."
        }
    }
}
