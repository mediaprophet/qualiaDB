package com.example.qualia.comms

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CallEnd
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material.icons.filled.Videocam
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.example.qualia.theme.BgDeep
import com.example.qualia.theme.NeonRed
import com.example.qualia.ui.components.PremiumButton

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SecureCallScreen(contactName: String, onEndCall: () -> Unit) {
    var isMuted by remember { mutableStateOf(false) }
    var isTranscribing by remember { mutableStateOf(true) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Secure Call: $contactName") },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = BgDeep)
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(BgDeep)
                .padding(padding),
            verticalArrangement = Arrangement.SpaceBetween,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            
            // Placeholder for native WebRTC SurfaceViewRenderer
            Box(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .background(Color.Black),
                contentAlignment = Alignment.Center
            ) {
                Text("Remote Video Stream (WebRTC)", color = Color.White)
                
                // Overlay for Desktop offloading notification
                Box(modifier = Modifier.align(Alignment.TopEnd).padding(16.dp)) {
                    Text("Desktop Compute Linked", color = Color.Green, style = MaterialTheme.typography.labelSmall)
                }
            }
            
            // Transcription View
            if (isTranscribing) {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.8f),
                    modifier = Modifier.fillMaxWidth().height(100.dp)
                ) {
                    Column(modifier = Modifier.padding(8.dp)) {
                        Text("Live Whisper Transcription (On-Device)", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                        Text("Hello, checking the secure channel...", style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }

            // Controls
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(24.dp),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically
            ) {
                FloatingActionButton(onClick = { isMuted = !isMuted }) {
                    Icon(if (isMuted) Icons.Default.MicOff else Icons.Default.Mic, contentDescription = "Mute")
                }
                
                FloatingActionButton(onClick = { /* Share CBOR-LD records */ }) {
                    Icon(Icons.Default.Share, contentDescription = "Share Health Data")
                }
                
                FloatingActionButton(
                    onClick = onEndCall,
                    containerColor = NeonRed,
                    contentColor = Color.White
                ) {
                    Icon(Icons.Default.CallEnd, contentDescription = "End Call")
                }
            }
        }
    }
}
