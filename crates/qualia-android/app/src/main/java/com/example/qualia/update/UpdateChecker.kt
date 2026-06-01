package com.example.qualia.update

import android.content.Context
import android.content.Intent
import android.net.Uri
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.net.URL

private const val MANIFEST_URL =
    "https://mediaprophet.github.io/qualiaDB/releases/latest.json"

@Serializable
data class ReleaseManifest(
    val version: String,
    val notes: String = "",
    val pub_date: String = "",
)

object UpdateChecker {

    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Returns the remote manifest if a newer version is available, null otherwise.
     * Never throws — network errors are silently swallowed (non-fatal).
     */
    suspend fun checkForUpdate(currentVersion: String): ReleaseManifest? =
        withContext(Dispatchers.IO) {
            runCatching {
                val raw = URL(MANIFEST_URL).readText()
                val manifest = json.decodeFromString<ReleaseManifest>(raw)
                if (isNewer(manifest.version, currentVersion)) manifest else null
            }.getOrNull()
        }

    /** Opens the GitHub releases page in the system browser. */
    fun openReleasePage(context: Context) {
        val intent = Intent(
            Intent.ACTION_VIEW,
            Uri.parse("https://github.com/mediaprophet/qualiaDB/releases/latest")
        )
        context.startActivity(intent)
    }

    // Simple semver comparison: "1.2.3" > "1.0.0"
    private fun isNewer(remote: String, current: String): Boolean {
        val r = remote.split(".").mapNotNull { it.toIntOrNull() }
        val c = current.split(".").mapNotNull { it.toIntOrNull() }
        for (i in 0 until maxOf(r.size, c.size)) {
            val rv = r.getOrElse(i) { 0 }
            val cv = c.getOrElse(i) { 0 }
            if (rv != cv) return rv > cv
        }
        return false
    }
}
