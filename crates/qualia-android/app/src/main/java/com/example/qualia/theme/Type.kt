package com.example.qualia.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

val QualiaTypography = Typography(
    displayLarge  = TextStyle(fontFamily = FontFamily.Default, fontWeight = FontWeight.Bold,   fontSize = 32.sp, color = NeonBlue),
    titleLarge    = TextStyle(fontFamily = FontFamily.Default, fontWeight = FontWeight.SemiBold,fontSize = 22.sp, color = TextPrimary),
    titleMedium   = TextStyle(fontFamily = FontFamily.Default, fontWeight = FontWeight.Medium,  fontSize = 16.sp, color = TextPrimary),
    bodyLarge     = TextStyle(fontFamily = FontFamily.Default, fontWeight = FontWeight.Normal,  fontSize = 16.sp, color = TextPrimary),
    bodyMedium    = TextStyle(fontFamily = FontFamily.Default, fontWeight = FontWeight.Normal,  fontSize = 14.sp, color = TextMuted),
    bodySmall     = TextStyle(fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Normal,fontSize = 12.sp, color = TextDim),
    labelLarge    = TextStyle(fontFamily = FontFamily.Default, fontWeight = FontWeight.Medium,  fontSize = 14.sp, color = NeonBlue),
    labelSmall    = TextStyle(fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Normal,fontSize = 11.sp, color = TextMuted),
)
