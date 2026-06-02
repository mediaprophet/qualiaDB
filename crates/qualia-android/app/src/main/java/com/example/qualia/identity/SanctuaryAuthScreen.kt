package com.example.qualia.identity

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.qualia.theme.BgDeep
import com.example.qualia.theme.NeonBlue
import com.example.qualia.theme.NeonRed
import com.example.qualia.theme.TextMuted

@Composable
fun SanctuaryAuthScreen(onAuthenticated: (Boolean) -> Unit) {
    var pin by remember { mutableStateOf("") }
    var isError by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDeep)
            .padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Icon(
            imageVector = Icons.Default.Lock,
            contentDescription = "Vault Lock",
            tint = NeonBlue,
            modifier = Modifier.size(64.dp)
        )
        
        Spacer(modifier = Modifier.height(16.dp))
        
        Text(
            text = "Qualia Vault",
            style = MaterialTheme.typography.headlineMedium,
            color = MaterialTheme.colorScheme.onBackground
        )
        Text(
            text = "Enter numerical PIN to access your datasets.",
            style = MaterialTheme.typography.bodyMedium,
            color = TextMuted,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(32.dp))

        // Simple PIN dots
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            for (i in 0 until 4) {
                Box(
                    modifier = Modifier
                        .size(16.dp)
                        .background(
                            color = if (i < pin.length) NeonBlue else TextMuted.copy(alpha = 0.3f),
                            shape = MaterialTheme.shapes.small
                        )
                )
            }
        }

        if (isError) {
            Spacer(modifier = Modifier.height(16.dp))
            Text("Invalid PIN.", color = NeonRed, fontSize = 12.sp)
        }

        Spacer(modifier = Modifier.height(48.dp))

        // Numpad
        val keys = listOf(
            listOf("1", "2", "3"),
            listOf("4", "5", "6"),
            listOf("7", "8", "9"),
            listOf("", "0", "DEL")
        )

        keys.forEach { row ->
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly
            ) {
                row.forEach { key ->
                    if (key.isEmpty()) {
                        Spacer(modifier = Modifier.size(64.dp))
                    } else {
                        TextButton(
                            onClick = {
                                if (key == "DEL") {
                                    if (pin.isNotEmpty()) pin = pin.dropLast(1)
                                    isError = false
                                } else {
                                    if (pin.length < 4) pin += key
                                    if (pin.length == 4) {
                                        // MOCK AUTHENTICATION LOGIC
                                        // 0000 = Demo Mode (Synthetic Personas)
                                        // 1234 = Standard/Decoy Lane
                                        // 9999 = Sanctuary Lane
                                        when (pin) {
                                            "0000" -> onAuthenticated(false) // Trigger DemoModeManager injection upstream
                                            "1234" -> onAuthenticated(false) // Not sanctuary
                                            "9999" -> onAuthenticated(true)  // Sanctuary active
                                            else -> {
                                                isError = true
                                                pin = ""
                                            }
                                        }
                                    }
                                }
                            },
                            modifier = Modifier.size(64.dp)
                        ) {
                            Text(
                                text = key,
                                fontSize = 24.sp,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onBackground
                            )
                        }
                    }
                }
            }
            Spacer(modifier = Modifier.height(8.dp))
        }
    }
}
