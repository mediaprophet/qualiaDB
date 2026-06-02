package com.example.qualia.spatial

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Foreground service to log GPS routes during an explicit "Tracking Session"
 * (e.g., starting a shift). This avoids restrictive Play Store background 
 * location bans while allowing continuous recording.
 */
class SpatialTrackingService : Service() {

    private val serviceScope = CoroutineScope(Dispatchers.IO)
    private var isTracking = false

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "START_TRACKING") {
            startForegroundService()
            startLogging()
        } else if (intent?.action == "STOP_TRACKING") {
            stopForeground(true)
            stopSelf()
        }
        return START_STICKY
    }

    private fun startForegroundService() {
        val channelId = "SpatialTrackingChannel"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Project Shift Tracking",
                NotificationManager.IMPORTANCE_LOW
            )
            val manager = getSystemService(NotificationManager::class.java)
            manager?.createNotificationChannel(channel)
        }

        val notification = NotificationCompat.Builder(this, channelId)
            .setContentTitle("Qualia Tracking Active")
            .setContentText("Logging route for Asset Apportionment...")
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .build()

        startForeground(1, notification)
    }

    private fun startLogging() {
        isTracking = true
        serviceScope.launch {
            // Mock Location polling
            // In a real scenario, this uses FusedLocationProviderClient
            while (isTracking) {
                // val location = locationClient.lastLocation...
                val mockLat = -33.8688
                val mockLng = 151.2093
                val timestamp = System.currentTimeMillis()
                
                // Construct a Spatial_Log JSON and push to Rust Core
                val spatialLogJson = """
                    {"lat": $mockLat, "lng": $mockLng, "ts": $timestamp}
                """.trimIndent()
                
                // QualiaCore.insertSpatialLog(spatialLogJson)
                
                delay(10_000) // Poll every 10 seconds
            }
        }
    }

    override fun onDestroy() {
        isTracking = false
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
