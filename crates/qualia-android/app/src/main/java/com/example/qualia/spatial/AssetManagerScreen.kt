package com.example.qualia.spatial

import android.content.Intent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AssetManagerScreen(onNavigateBack: () -> Unit) {
    val context = LocalContext.current
    var isTracking by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Asset Manager") },
                navigationIcon = {
                    Button(onClick = onNavigateBack) {
                        Text("< Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Text("Define Physical Assets for Apportionment", style = MaterialTheme.typography.titleMedium)

            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Asset: Work Vehicle (Toyota Hilux)", fontWeight = androidx.compose.ui.text.font.FontWeight.Bold)
                    Text("Type: Mobile Asset")
                    // In a real app, this would open a map to draw a geofence or bind to an OBD2 sensor
                }
            }

            Spacer(modifier = Modifier.height(32.dp))

            Button(
                onClick = {
                    val intent = Intent(context, SpatialTrackingService::class.java).apply {
                        action = if (isTracking) "STOP_TRACKING" else "START_TRACKING"
                    }
                    if (isTracking) {
                        context.stopService(intent)
                    } else {
                        context.startService(intent) // Start Foreground Service
                    }
                    isTracking = !isTracking
                },
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(if (isTracking) "Stop Shift Tracking" else "Start Shift Tracking (GPS)")
            }
        }
    }
}
