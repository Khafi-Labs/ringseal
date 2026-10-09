package com.ringseal.sdk

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okhttp3.CertificatePinner
import okhttp3.ConnectionSpec
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.TlsVersion
import java.io.IOException
import java.util.concurrent.TimeUnit

class RingsealClient(private val config: RingsealConfig) {
    private val client: OkHttpClient

    init {
        val spec = ConnectionSpec.Builder(ConnectionSpec.MODERN_TLS)
            .tlsVersions(TlsVersion.TLS_1_2, TlsVersion.TLS_1_3)
            .build()
            
        val certificatePinner = CertificatePinner.Builder()
            // TODO: Replace with actual pins in production
            .add(config.baseUrl.removePrefix("https://").removePrefix("http://").substringBefore("/"), "sha256/PLACEHOLDER_PIN_DO_NOT_USE_IN_PROD")
            .build()

        client = OkHttpClient.Builder()
            .connectTimeout(config.connectTimeoutMs, TimeUnit.MILLISECONDS)
            .readTimeout(config.readTimeoutMs, TimeUnit.MILLISECONDS)
            .connectionSpecs(listOf(spec, ConnectionSpec.CLEARTEXT))
            .certificatePinner(certificatePinner)
            .build()
    }

    private var authToken: String? = null

    private val json = Json { ignoreUnknownKeys = true }

    fun setAuthToken(token: String) {
        authToken = token
    }

    suspend fun getActiveSession(): VerificationSession? = withContext(Dispatchers.IO) {
        val token = authToken ?: throw RingsealException("MISSING_TOKEN", "Authentication token is not set")
        
        val url = "${config.baseUrl.trimEnd('/')}/api/v1/verify"
        val request = Request.Builder()
            .url(url)
            .header("Authorization", "Bearer $token")
            .get()
            .build()

        try {
            client.newCall(request).execute().use { response ->
                when {
                    response.isSuccessful -> {
                        val bodyString = response.body?.string() ?: return@use null
                        return@withContext json.decodeFromString<VerificationSession>(bodyString)
                    }
                    response.code == 404 -> {
                        return@withContext null
                    }
                    else -> {
                        val errorBody = response.body?.string()
                        val errorDetail = try {
                            errorBody?.let { json.decodeFromString<RingsealErrorResponse>(it).error }
                        } catch (e: Exception) {
                            null
                        }
                        
                        throw RingsealException(
                            errorDetail?.code ?: "HTTP_ERROR",
                            errorDetail?.message ?: "HTTP Error: ${response.code}"
                        )
                    }
                }
            }
        } catch (e: IOException) {
            throw RingsealException("NETWORK_ERROR", "Network request failed: ${e.message}")
        }
    }

    fun shutdown() {
        client.dispatcher.executorService.shutdown()
        client.connectionPool.evictAll()
    }
}
