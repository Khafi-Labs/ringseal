package com.ringseal.sdk

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
enum class SessionStatus {
    @SerialName("created") CREATED,
    @SerialName("active") ACTIVE,
    @SerialName("verified") VERIFIED,
    @SerialName("expired") EXPIRED,
    @SerialName("ended") ENDED
}

@Serializable
data class VerificationSession(
    @SerialName("session_id") val sessionId: String,
    val code: String,
    @SerialName("expires_at") val expiresAt: String,
    val status: SessionStatus
)

@Serializable
data class RingsealErrorResponse(
    val error: RingsealErrorDetail
)

@Serializable  
data class RingsealErrorDetail(
    val code: String,
    val message: String
)

class RingsealException(val errorCode: String, message: String) : Exception(message)
