package com.example.qualia.memes

import android.content.Context
import android.graphics.Bitmap
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.ParcelFileDescriptor
import androidx.compose.runtime.Stable
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import java.security.MessageDigest

// ── Data model ───────────────────────────────────────────────────────────────

@Serializable
@Stable
data class MemeEntry(
    val id: String,                   // SHA-256 of file bytes
    val fileUri: String,              // absolute file:// path
    val mimeType: String = "image/*",
    val importedAt: Long = System.currentTimeMillis(),
    // LLM-generated semantic fields (null until indexed)
    val topic: String? = null,
    val emotion: String? = null,
    val caption: String? = null,
    val useWhen: List<String> = emptyList(),
    val confidence: Float = 0f,
    val indexed: Boolean = false,
)

// ── N-Quad serialisation ─────────────────────────────────────────────────────

fun MemeEntry.toNQuads(): String = buildString {
    val s = ":meme_$id"
    appendLine("<<$s rdf:type :Meme>> :imported_at \"$importedAt\" .")
    appendLine("<<$s :file_uri \"$fileUri\">>")
    appendLine("<<$s :mime_type \"$mimeType\">>")
    topic?.let   { appendLine("<<$s :topic    :${it.replace(" ","_")}>> :confidence $confidence .") }
    emotion?.let { appendLine("<<$s :emotion  :${it.replace(" ","_")}>> :confidence $confidence .") }
    caption?.let { appendLine("<<$s :caption  \"${it.replace("\"","'")}\">>") }
    useWhen.forEach { uw ->
        appendLine("<<$s :use_when \"${uw.replace("\"","'")}\">>")
    }
    if (indexed) appendLine("<<$s :indexed_by :LocalLlm>>")
}

// ── Library ──────────────────────────────────────────────────────────────────

class MemeLibrary(private val context: Context) {

    private val storeFile: File
        get() = File(context.filesDir, "meme_library.json")

    private val _memes = MutableStateFlow<List<MemeEntry>>(emptyList())
    val memes: StateFlow<List<MemeEntry>> = _memes

    private val json = Json { ignoreUnknownKeys = true; prettyPrint = false }

    // ── Persistence ───────────────────────────────────────────────────────────

    suspend fun load() = withContext(Dispatchers.IO) {
        runCatching {
            if (storeFile.exists()) {
                val list = json.decodeFromString<List<MemeEntry>>(storeFile.readText())
                _memes.value = list
            }
        }
    }

    private suspend fun persist() = withContext(Dispatchers.IO) {
        storeFile.writeText(json.encodeToString(_memes.value))
    }

    // ── Import ────────────────────────────────────────────────────────────────

    /** Import an image URI from the gallery / camera. Returns the new MemeEntry. */
    suspend fun importImage(uri: Uri, mimeType: String = "image/*"): MemeEntry =
        withContext(Dispatchers.IO) {
            val bytes = context.contentResolver.openInputStream(uri)!!.readBytes()
            val sha256 = sha256Hex(bytes)

            // Copy to private storage so URI stays valid after gallery changes
            val destFile = File(context.filesDir, "memes/$sha256.img").also {
                it.parentFile?.mkdirs()
                if (!it.exists()) it.writeBytes(bytes)
            }

            val entry = MemeEntry(
                id       = sha256,
                fileUri  = destFile.absolutePath,
                mimeType = mimeType,
            )
            addEntry(entry)
            entry
        }

    private suspend fun addEntry(entry: MemeEntry) {
        if (_memes.value.none { it.id == entry.id }) {
            _memes.value = _memes.value + entry
            persist()
        }
    }

    /** Update a meme with LLM-generated semantic metadata. */
    suspend fun updateWithSemantics(
        id: String,
        topic: String,
        emotion: String,
        caption: String,
        useWhen: List<String>,
        confidence: Float,
    ) {
        _memes.value = _memes.value.map { m ->
            if (m.id == id) m.copy(
                topic      = topic,
                emotion    = emotion,
                caption    = caption,
                useWhen    = useWhen,
                confidence = confidence,
                indexed    = true,
            ) else m
        }
        persist()
    }

    /** Delete a meme from the library. */
    suspend fun delete(id: String) {
        val entry = _memes.value.find { it.id == id } ?: return
        File(entry.fileUri).delete()
        _memes.value = _memes.value.filter { it.id != id }
        persist()
    }

    // ── Search ────────────────────────────────────────────────────────────────

    /**
     * Keyword search: matches against topic, emotion, caption, and useWhen.
     * Returns entries ranked by number of field hits (descending).
     */
    fun search(query: String): List<Pair<MemeEntry, Int>> {
        if (query.isBlank()) return _memes.value.map { it to 0 }
        val terms = query.lowercase().split(Regex("\\s+"))
        return _memes.value
            .map { m ->
                val haystack = listOfNotNull(m.topic, m.emotion, m.caption)
                    .plus(m.useWhen)
                    .joinToString(" ")
                    .lowercase()
                val hits = terms.count { t -> haystack.contains(t) }
                m to hits
            }
            .filter { (_, hits) -> hits > 0 }
            .sortedByDescending { (_, hits) -> hits }
    }

    /** Returns all unindexed memes (LLM hasn't run on them yet). */
    fun unindexed(): List<MemeEntry> = _memes.value.filter { !it.indexed }

    // ── Utilities ─────────────────────────────────────────────────────────────

    private fun sha256Hex(bytes: ByteArray): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(bytes)
        return digest.joinToString("") { "%02x".format(it) }
    }

    /** Export the full library as N-Quad text (.nq file). */
    fun exportNQuads(): String = _memes.value.joinToString("\n") { it.toNQuads() }
}
