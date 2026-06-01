package com.example.qualia.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val QualiaColorScheme = darkColorScheme(
    primary          = NeonBlue,
    onPrimary        = BgDeep,
    primaryContainer = BorderGlow,
    secondary        = NeonGold,
    onSecondary      = BgDeep,
    tertiary         = NeonPurple,
    background       = BgDeep,
    surface          = BgCard,
    surfaceVariant   = BgGlass,
    onBackground     = TextPrimary,
    onSurface        = TextPrimary,
    onSurfaceVariant = TextMuted,
    outline          = BorderDim,
    error            = NeonRed,
)

@Composable
fun QualiaTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = QualiaColorScheme,
        typography  = QualiaTypography,
        content     = content,
    )
}
