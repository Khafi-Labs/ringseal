package com.ringseal.sdk.ui

import java.io.Serializable

data class RingsealTheme(
    val primaryColor: Int = 0xFF1A73E8.toInt(),
    val backgroundColor: Int = 0xFFF8F9FA.toInt(),
    val codeBoxColor: Int = 0xFFFFFFFF.toInt(),
    val codeTextColor: Int = 0xFF202124.toInt(),
    val warningColor: Int = 0xFFEA4335.toInt(),
    val titleTextSize: Float = 20f,
    val codeTextSize: Float = 32f
) : Serializable
