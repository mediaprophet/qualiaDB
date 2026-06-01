package com.example.qualia.memes

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.floatOrNull

/**
 * Drives LLM-based semantic tagging of meme images.
 *
 * The indexer:
 * 1. Loads each unindexed meme bitmap (512×512 thumbnail)
 * 2. Sends a structured prompt to the on-device LLM asking for JSON output
 * 3. Parses the JSON → updates the MemeLibrary with semantic tags
 * 4. Tags are stored as N-Quads via MemeLibrary.updateWithSemantics()
 *
 * The actual LLM call is delegated to LlmEngine (loaded separately).
 * If the LLM is not yet loaded, indexing is a no-op and retried next session.
 */
class MemeIndexer(
    private val library: MemeLibrary,
    private val llmInference: suspend (prompt: String) -> String,
) {
    private val json = Json { ignoreUnknownKeys = true }

    companion object {
        val INDEXING_PROMPT = """
You are a meme classification assistant. Given a description or visible text from a meme image, output ONLY valid JSON with this exact schema:
{
  "topic": "<one short noun phrase>",
  "emotion": "<one emotion word>",
  "caption": "<verbatim text visible in the meme, or empty string>",
  "use_when": ["<phrase 1>", "<phrase 2>", "<phrase 3>"],
  "confidence": <float 0.0-1.0>
}
No other text. No markdown. Only the JSON object.
        """.trimIndent()
    }

    /**
     * Index all unindexed memes in the library.
     * [imageDescriber] is a suspend function that accepts a file path and
     * returns a text description of the image (from a vision model or OCR).
     */
    suspend fun indexAll(
        imageDescriber: suspend (filePath: String) -> String,
        onProgress: (indexed: Int, total: Int) -> Unit = { _, _ -> },
    ) = withContext(Dispatchers.Default) {
        val pending = library.unindexed()
        pending.forEachIndexed { i, meme ->
            runCatching {
                // Step 1: get a text description of the image
                val imageDesc = imageDescriber(meme.fileUri)

                // Step 2: prompt LLM for structured JSON tags
                val prompt = "$INDEXING_PROMPT\n\nImage description:\n$imageDesc"
                val rawJson = llmInference(prompt)

                // Step 3: parse JSON
                val root = json.parseToJsonElement(rawJson).jsonObject
                val topic      = root["topic"]?.jsonPrimitive?.content ?: "unknown"
                val emotion    = root["emotion"]?.jsonPrimitive?.content ?: "neutral"
                val caption    = root["caption"]?.jsonPrimitive?.content ?: ""
                val useWhen    = root["use_when"]?.jsonArray
                    ?.map { it.jsonPrimitive.content } ?: emptyList()
                val confidence = root["confidence"]?.jsonPrimitive?.floatOrNull ?: 0.7f

                // Step 4: persist
                library.updateWithSemantics(
                    id         = meme.id,
                    topic      = topic,
                    emotion    = emotion,
                    caption    = caption,
                    useWhen    = useWhen,
                    confidence = confidence,
                )
            } // silently skip if LLM returns garbage — retry next session

            onProgress(i + 1, pending.size)
        }
    }
}
