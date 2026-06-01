package com.example.qualia.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material.icons.filled.EmojiEmotions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.example.qualia.memes.MemeEntry
import com.example.qualia.memes.MemePickerSheet
import com.example.qualia.memes.MemeViewModel
import com.example.qualia.theme.*

// ── Message model ─────────────────────────────────────────────────────────────

sealed class ChatMessage {
    abstract val id: String
    data class User(override val id: String, val text: String,   val imageUri: String? = null) : ChatMessage()
    data class Bot (override val id: String, val text: String,   val isStreaming: Boolean = false) : ChatMessage()
    data class Meme(override val id: String, val entry: MemeEntry) : ChatMessage()
    data class MemeSuggestion(override val id: String, val entry: MemeEntry, val reason: String) : ChatMessage()
}

// ── Screen ────────────────────────────────────────────────────────────────────

@Composable
fun ChatScreen(
    viewModel: ChatViewModel,
    memeViewModel: MemeViewModel,
) {
    val messages      by viewModel.messages.collectAsState()
    val isGenerating  by viewModel.isGenerating.collectAsState()
    var inputText     by remember { mutableStateOf("") }
    var showMemePicker by remember { mutableStateOf(false) }
    val listState     = rememberLazyListState()

    // Scroll to bottom when new messages arrive
    LaunchedEffect(messages.size) {
        if (messages.isNotEmpty()) listState.animateScrollToItem(messages.size - 1)
    }

    Column(
        Modifier
            .fillMaxSize()
            .background(BgDeep)
            .imePadding()
    ) {
        // ── Header ─────────────────────────────────────────────────────────────
        Row(
            Modifier
                .fillMaxWidth()
                .background(
                    Brush.horizontalGradient(listOf(BgCard, BgDeep))
                )
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                Modifier.size(10.dp).clip(CircleShape)
                    .background(if (isGenerating) NeonGold else NeonGreen)
            )
            Spacer(Modifier.width(8.dp))
            Text(
                "QUALIA CHAT",
                style = MaterialTheme.typography.displayLarge.copy(fontSize = 16.sp),
            )
            Spacer(Modifier.weight(1f))
            Text(
                if (isGenerating) "generating…" else "on-device · private",
                color = TextMuted, fontSize = 11.sp,
            )
        }

        HorizontalDivider(color = BorderDim)

        // ── Message thread ─────────────────────────────────────────────────────
        LazyColumn(
            state          = listState,
            modifier       = Modifier.weight(1f).fillMaxWidth(),
            contentPadding = PaddingValues(12.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            if (messages.isEmpty()) {
                item {
                    WelcomePlaceholder()
                }
            }
            items(messages, key = { it.id }) { msg ->
                when (msg) {
                    is ChatMessage.User ->
                        UserBubble(msg)
                    is ChatMessage.Bot ->
                        BotBubble(msg)
                    is ChatMessage.Meme ->
                        MemeBubble(msg.entry)
                    is ChatMessage.MemeSuggestion ->
                        MemeSuggestionCard(
                            msg,
                            onAccept = { viewModel.acceptMemeSuggestion(msg) },
                            onDismiss = { viewModel.dismissMemeSuggestion(msg) },
                        )
                }
            }
        }

        // ── Input bar ──────────────────────────────────────────────────────────
        Column(
            Modifier
                .fillMaxWidth()
                .background(BgCard)
                .border(1.dp, BorderDim, RoundedCornerShape(topStart = 12.dp, topEnd = 12.dp))
                .padding(horizontal = 12.dp, vertical = 8.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                // Meme picker button
                IconButton(onClick = { showMemePicker = true }) {
                    Icon(Icons.Default.EmojiEmotions, contentDescription = "Find meme",
                        tint = NeonGold)
                }
                // Attach image / PDF
                IconButton(onClick = { /* TODO: file picker */ }) {
                    Icon(Icons.Default.AttachFile, contentDescription = "Attach file",
                        tint = TextMuted)
                }

                OutlinedTextField(
                    value         = inputText,
                    onValueChange = { inputText = it },
                    placeholder   = { Text("Message…", color = TextDim) },
                    modifier      = Modifier.weight(1f),
                    shape         = RoundedCornerShape(20.dp),
                    colors        = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor   = NeonBlue,
                        unfocusedBorderColor = BorderDim,
                        focusedTextColor     = TextPrimary,
                        unfocusedTextColor   = TextPrimary,
                        cursorColor          = NeonBlue,
                    ),
                    maxLines = 4,
                )

                IconButton(
                    onClick = {
                        val t = inputText.trim()
                        if (t.isNotEmpty()) {
                            viewModel.sendMessage(t)
                            inputText = ""
                        }
                    },
                    enabled = inputText.isNotBlank() && !isGenerating,
                ) {
                    Icon(
                        Icons.AutoMirrored.Filled.Send,
                        contentDescription = "Send",
                        tint = if (inputText.isNotBlank() && !isGenerating) NeonBlue else TextDim,
                    )
                }
            }
            Text(
                "All inference is on-device. No data leaves your phone.",
                color = TextDim, fontSize = 10.sp,
                modifier = Modifier.padding(top = 4.dp, start = 4.dp),
            )
        }
    }

    // ── Meme Picker sheet ──────────────────────────────────────────────────────
    if (showMemePicker) {
        MemePickerSheet(
            memeViewModel  = memeViewModel,
            prefilledQuery = viewModel.currentTopic(),
            onMemeSelected = { entry ->
                viewModel.insertMeme(entry)
                showMemePicker = false
            },
            onDismiss = { showMemePicker = false },
        )
    }
}

// ── Bubble composables ────────────────────────────────────────────────────────

@Composable
private fun UserBubble(msg: ChatMessage.User) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
        Column(
            Modifier
                .widthIn(max = 280.dp)
                .background(
                    Brush.linearGradient(listOf(NeonBlue.copy(0.25f), NeonPurple.copy(0.15f))),
                    RoundedCornerShape(16.dp, 4.dp, 16.dp, 16.dp)
                )
                .border(1.dp, NeonBlue.copy(0.3f), RoundedCornerShape(16.dp, 4.dp, 16.dp, 16.dp))
                .padding(12.dp)
        ) {
            msg.imageUri?.let {
                AsyncImage(it, contentDescription = null,
                    modifier = Modifier.fillMaxWidth().heightIn(max = 180.dp)
                        .clip(RoundedCornerShape(8.dp)),
                    contentScale = ContentScale.Crop,
                )
                Spacer(Modifier.height(6.dp))
            }
            Text(msg.text, color = TextPrimary, fontSize = 14.sp)
        }
    }
}

@Composable
private fun BotBubble(msg: ChatMessage.Bot) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Start) {
        Column(
            Modifier
                .widthIn(max = 300.dp)
                .background(BgCard, RoundedCornerShape(4.dp, 16.dp, 16.dp, 16.dp))
                .border(1.dp, BorderDim, RoundedCornerShape(4.dp, 16.dp, 16.dp, 16.dp))
                .padding(12.dp)
        ) {
            Text(msg.text, color = TextPrimary, fontSize = 14.sp)
            if (msg.isStreaming) {
                Spacer(Modifier.height(4.dp))
                LinearProgressIndicator(
                    modifier = Modifier.fillMaxWidth().height(1.dp),
                    color = NeonBlue,
                    trackColor = BorderDim,
                )
            }
        }
    }
}

@Composable
private fun MemeBubble(entry: MemeEntry) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
        Box(
            Modifier
                .widthIn(max = 240.dp)
                .clip(RoundedCornerShape(12.dp))
                .border(1.dp, NeonGold.copy(0.4f), RoundedCornerShape(12.dp))
        ) {
            AsyncImage(
                model             = entry.fileUri,
                contentDescription = entry.caption ?: "meme",
                modifier          = Modifier.fillMaxWidth(),
                contentScale      = ContentScale.FillWidth,
            )
        }
    }
}

@Composable
private fun MemeSuggestionCard(
    msg: ChatMessage.MemeSuggestion,
    onAccept: () -> Unit,
    onDismiss: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp),
        colors   = CardDefaults.cardColors(containerColor = BgCard),
        shape    = RoundedCornerShape(10.dp),
    ) {
        Row(
            Modifier.padding(10.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment     = Alignment.CenterVertically,
        ) {
            AsyncImage(
                model             = msg.entry.fileUri,
                contentDescription = msg.entry.caption,
                modifier          = Modifier.size(60.dp).clip(RoundedCornerShape(6.dp)),
                contentScale      = ContentScale.Crop,
            )
            Column(Modifier.weight(1f)) {
                Text("Suggested meme", color = NeonGold, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                Text(msg.reason, color = TextMuted, fontSize = 12.sp, maxLines = 2)
            }
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                TextButton(onClick = onAccept, contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp)) {
                    Text("Use", color = NeonBlue, fontSize = 12.sp)
                }
                TextButton(onClick = onDismiss, contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp)) {
                    Text("Skip", color = TextMuted, fontSize = 12.sp)
                }
            }
        }
    }
}

@Composable
private fun WelcomePlaceholder() {
    Box(Modifier.fillMaxWidth().padding(top = 60.dp), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("🧠", fontSize = 48.sp)
            Text("QUALIA ON-DEVICE CHAT", color = NeonBlue, fontSize = 14.sp, fontWeight = FontWeight.Bold)
            Text("All inference runs locally.", color = TextMuted, fontSize = 12.sp)
            Text("Your data never leaves your phone.", color = TextDim, fontSize = 11.sp)
        }
    }
}
