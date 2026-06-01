package com.example.qualia.llm

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.withContext
import java.io.File
import java.net.URL
import java.security.MessageDigest

// ── Model catalogue ───────────────────────────────────────────────────────────

enum class ModelTier(
    val displayName:   String,
    val paramBillions: Float,
    val sizeMb:        Int,
    val minRamMb:      Int,
    val sha256:        String,
    val downloadUrl:   String,
) {
    TINY_LLAMA(
        displayName   = "TinyLlama 1.1B (Q4_K_M)",
        paramBillions = 1.1f,
        sizeMb        = 638,
        minRamMb      = 1500,
        sha256        = "",   // filled at release; verified before load
        downloadUrl   = "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
    ),
    GEMMA_2B(
        displayName   = "Gemma 2B (Q4_K)",
        paramBillions = 2.0f,
        sizeMb        = 1500,
        minRamMb      = 3000,
        sha256        = "",
        downloadUrl   = "https://huggingface.co/lmstudio-ai/gemma-2b-it-GGUF/resolve/main/gemma-2b-it-q4_k_m.gguf",
    ),
    PHI3_MINI(
        displayName   = "Phi-3 Mini 3.8B (Q4_K_M)",
        paramBillions = 3.8f,
        sizeMb        = 2300,
        minRamMb      = 4500,
        sha256        = "",
        downloadUrl   = "https://huggingface.co/bartowski/Phi-3-mini-4k-instruct-GGUF/resolve/main/Phi-3-mini-4k-instruct-Q4_K_M.gguf",
    );

    fun modelFile(context: Context): File =
        File(context.filesDir, "models/${name.lowercase()}.gguf")

    fun isDownloaded(context: Context): Boolean = modelFile(context).exists()
}

// ── Download progress ─────────────────────────────────────────────────────────

sealed class DownloadState {
    object Idle                                    : DownloadState()
    data class Downloading(val progress: Float)    : DownloadState()
    data class Verifying(val progress: Float)      : DownloadState()
    object Ready                                   : DownloadState()
    data class Error(val message: String)          : DownloadState()
}

// ── ModelDownloader ───────────────────────────────────────────────────────────

/**
 * Downloads and SHA-256 verifies a model GGUF file for on-device LLM inference.
 *
 * Download goes to app-private storage (no external storage permissions needed).
 * Verification is skipped if the expected SHA-256 is empty (dev/test builds).
 *
 * Emits [DownloadState] events as a Flow so the UI can show real progress.
 */
class ModelDownloader(private val context: Context) {

    fun download(tier: ModelTier): Flow<DownloadState> = flow {
        val dest = tier.modelFile(context).also { it.parentFile?.mkdirs() }

        // Already have it?
        if (dest.exists()) {
            emit(DownloadState.Ready)
            return@flow
        }

        emit(DownloadState.Downloading(0f))

        runCatching {
            withContext(Dispatchers.IO) {
                val url  = URL(tier.downloadUrl)
                val conn = url.openConnection().also {
                    it.connectTimeout = 15_000
                    it.readTimeout    = 60_000
                    it.connect()
                }
                val total  = conn.contentLengthLong.takeIf { it > 0 } ?: (tier.sizeMb * 1_048_576L)
                var written = 0L

                conn.getInputStream().use { input ->
                    dest.outputStream().use { output ->
                        val buf = ByteArray(8192)
                        var n: Int
                        while (input.read(buf).also { n = it } >= 0) {
                            output.write(buf, 0, n)
                            written += n
                            emit(DownloadState.Downloading(written.toFloat() / total))
                        }
                    }
                }
            }
        }.onFailure { e ->
            dest.delete()
            emit(DownloadState.Error("Download failed: ${e.message}"))
            return@flow
        }

        // SHA-256 verification
        if (tier.sha256.isNotEmpty()) {
            emit(DownloadState.Verifying(0f))
            val actual = withContext(Dispatchers.IO) { sha256File(dest) { p -> } }
            if (!actual.equals(tier.sha256, ignoreCase = true)) {
                dest.delete()
                emit(DownloadState.Error("SHA-256 mismatch — file corrupted or tampered"))
                return@flow
            }
        }

        Log.i("ModelDownloader", "Model ready: ${dest.absolutePath}")
        emit(DownloadState.Ready)
    }

    fun delete(tier: ModelTier) = tier.modelFile(context).delete()

    private fun sha256File(file: File, onProgress: (Float) -> Unit): String {
        val md    = MessageDigest.getInstance("SHA-256")
        val total = file.length().toFloat()
        var read  = 0L
        file.inputStream().use { input ->
            val buf = ByteArray(65536)
            var n: Int
            while (input.read(buf).also { n = it } >= 0) {
                md.update(buf, 0, n)
                read += n
                onProgress(read / total)
            }
        }
        return md.digest().joinToString("") { "%02x".format(it) }
    }
}
