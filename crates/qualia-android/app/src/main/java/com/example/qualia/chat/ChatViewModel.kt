package com.example.qualia.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.qualia.memes.MemeEntry
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import java.util.UUID

class ChatViewModel : ViewModel() {

    private val _messages = MutableStateFlow<List<ChatMessage>>(emptyList())
    val messages: StateFlow<List<ChatMessage>> = _messages

    private val _isGenerating = MutableStateFlow(false)
    val isGenerating: StateFlow<Boolean> = _isGenerating

    // Injected later when LLM is loaded
    var llmInference: (suspend (String) -> Flow<String>)? = null
    var memeSearchFn: ((String) -> MemeEntry?)? = null

    fun sendMessage(text: String) {
        val userMsg = ChatMessage.User(UUID.randomUUID().toString(), text)
        _messages.value = _messages.value + userMsg

        viewModelScope.launch {
            _isGenerating.value = true

            // Check if the LLM wants to suggest a meme
            memeSearchFn?.let { search ->
                val topicKeywords = extractTopicKeywords(text)
                if (topicKeywords.isNotBlank()) {
                    search(topicKeywords)?.let { meme ->
                        val suggestion = ChatMessage.MemeSuggestion(
                            id     = UUID.randomUUID().toString(),
                            entry  = meme,
                            reason = "Relevant to: \"$topicKeywords\"",
                        )
                        _messages.value = _messages.value + suggestion
                    }
                }
            }

            // Stream the LLM response
            val botId   = UUID.randomUUID().toString()
            val botMsg  = ChatMessage.Bot(botId, "", isStreaming = true)
            _messages.value = _messages.value + botMsg

            val inference = llmInference
            if (inference != null) {
                val sb = StringBuilder()
                inference(buildPrompt(text)).collect { token ->
                    sb.append(token)
                    _messages.value = _messages.value.map { m ->
                        if (m.id == botId) ChatMessage.Bot(botId, sb.toString(), isStreaming = true)
                        else m
                    }
                }
                // Mark done
                _messages.value = _messages.value.map { m ->
                    if (m.id == botId) ChatMessage.Bot(botId, sb.toString(), isStreaming = false)
                    else m
                }
            } else {
                // LLM not loaded — placeholder
                _messages.value = _messages.value.map { m ->
                    if (m.id == botId) ChatMessage.Bot(
                        botId,
                        "⚠️ LLM model not loaded. Go to Settings → Download Model to enable on-device inference.",
                        isStreaming = false,
                    ) else m
                }
            }

            _isGenerating.value = false
        }
    }

    fun insertMeme(entry: MemeEntry) {
        _messages.value = _messages.value +
            ChatMessage.Meme(UUID.randomUUID().toString(), entry)
    }

    fun acceptMemeSuggestion(msg: ChatMessage.MemeSuggestion) {
        _messages.value = _messages.value
            .filterNot { it.id == msg.id } +
            ChatMessage.Meme(UUID.randomUUID().toString(), msg.entry)
    }

    fun dismissMemeSuggestion(msg: ChatMessage.MemeSuggestion) {
        _messages.value = _messages.value.filterNot { it.id == msg.id }
    }

    /** Returns the most recent user message as a topic hint for the meme picker. */
    fun currentTopic(): String =
        (_messages.value.filterIsInstance<ChatMessage.User>().lastOrNull()?.text ?: "")
            .take(60)

    // ── Helpers ───────────────────────────────────────────────────────────────

    private fun buildPrompt(userText: String): String {
        val history = _messages.value
            .takeLast(10)
            .filterIsInstance<ChatMessage.User>()
            .joinToString("\n") { "User: ${it.text}" }
        return "$history\nUser: $userText\nAssistant:"
    }

    /**
     * Naively extracts potential meme-relevant keywords from the user's text.
     * In full implementation this would call the LLM for intent extraction.
     */
    private fun extractTopicKeywords(text: String): String {
        val emotionWords = setOf(
            "shocked", "surprised", "angry", "sad", "happy", "confused",
            "disappointed", "tired", "excited", "annoyed", "worried",
        )
        return text.lowercase().split(Regex("\\W+"))
            .filter { it.length > 3 && (it in emotionWords || it.length > 5) }
            .take(3)
            .joinToString(" ")
    }
}
