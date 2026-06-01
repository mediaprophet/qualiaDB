package com.example.qualia.llm

import androidx.compose.animation.core.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.qualia.theme.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

// ── ViewModel ─────────────────────────────────────────────────────────────────

class LlmViewModel : ViewModel() {

    private val _downloadState = MutableStateFlow<DownloadState>(DownloadState.Idle)
    val downloadState: StateFlow<DownloadState> = _downloadState

    private val _selectedTier = MutableStateFlow(ModelTier.TINY_LLAMA)
    val selectedTier: StateFlow<ModelTier> = _selectedTier

    private val _availableModels = MutableStateFlow<Set<ModelTier>>(emptySet())
    val availableModels: StateFlow<Set<ModelTier>> = _availableModels

    fun selectTier(tier: ModelTier) { _selectedTier.value = tier }

    fun checkDownloaded(context: android.content.Context) {
        _availableModels.value = ModelTier.entries.filter { it.isDownloaded(context) }.toSet()
    }

    fun download(context: android.content.Context) {
        val tier = _selectedTier.value
        viewModelScope.launch {
            ModelDownloader(context).download(tier).collect { state ->
                _downloadState.value = state
                if (state is DownloadState.Ready) checkDownloaded(context)
            }
        }
    }

    fun delete(context: android.content.Context, tier: ModelTier) {
        ModelDownloader(context).delete(tier)
        checkDownloaded(context)
        _downloadState.value = DownloadState.Idle
    }
}

// ── Screen ────────────────────────────────────────────────────────────────────

@Composable
fun LlmScreen(viewModel: LlmViewModel) {
    val context     = LocalContext.current
    val dlState     by viewModel.downloadState.collectAsState()
    val selected    by viewModel.selectedTier.collectAsState()
    val available   by viewModel.availableModels.collectAsState()

    LaunchedEffect(Unit) { viewModel.checkDownloaded(context) }

    Column(
        Modifier
            .fillMaxSize()
            .background(BgDeep)
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        // Header
        Text("ON-DEVICE LLM", style = MaterialTheme.typography.displayLarge.copy(fontSize = 18.sp))
        Text("All inference runs locally. No data leaves your device.",
            color = TextMuted, fontSize = 13.sp)

        // Privacy badge
        Card(
            colors   = CardDefaults.cardColors(containerColor = NeonGreen.copy(0.06f)),
            shape    = RoundedCornerShape(10.dp),
            border   = BorderStroke(1.dp, NeonGreen.copy(0.3f)),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Row(
                Modifier.padding(12.dp),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalAlignment     = Alignment.CenterVertically,
            ) {
                Icon(Icons.Default.Shield, null, tint = NeonGreen, modifier = Modifier.size(22.dp))
                Column {
                    Text("Privacy-first inference", color = NeonGreen, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                    Text("LLM Governance Rules enforced · Fiduciary Supremacy active · No telemetry",
                        color = TextMuted, fontSize = 11.sp)
                }
            }
        }

        // Model selection
        Text("Select model tier", color = TextMuted, fontSize = 13.sp, fontWeight = FontWeight.Medium)
        ModelTier.entries.forEach { tier ->
            ModelTierCard(
                tier      = tier,
                selected  = selected == tier,
                available = tier in available,
                onSelect  = { viewModel.selectTier(tier) },
                onDelete  = { viewModel.delete(context, tier) },
            )
        }

        // Download/status section
        Spacer(Modifier.height(4.dp))

        when (val s = dlState) {
            is DownloadState.Idle -> {
                if (selected !in available) {
                    Button(
                        onClick  = { viewModel.download(context) },
                        colors   = ButtonDefaults.buttonColors(
                            containerColor = NeonBlue, contentColor = BgDeep,
                        ),
                        modifier = Modifier.fillMaxWidth(),
                        shape    = RoundedCornerShape(8.dp),
                    ) {
                        Icon(Icons.Default.Download, null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("Download ${selected.displayName}", fontWeight = FontWeight.Bold)
                    }
                    Text(
                        "~${selected.sizeMb} MB · requires ${selected.minRamMb} MB RAM",
                        color = TextMuted, fontSize = 11.sp,
                        modifier = Modifier.align(Alignment.CenterHorizontally),
                    )
                } else {
                    ReadyBadge(selected)
                }
            }
            is DownloadState.Downloading -> {
                ProgressSection("Downloading…", s.progress, NeonBlue)
            }
            is DownloadState.Verifying -> {
                ProgressSection("Verifying SHA-256…", s.progress, NeonGold)
            }
            is DownloadState.Ready -> {
                ReadyBadge(selected)
            }
            is DownloadState.Error -> {
                Card(
                    colors   = CardDefaults.cardColors(containerColor = NeonRed.copy(0.08f)),
                    border   = BorderStroke(1.dp, NeonRed.copy(0.4f)),
                    shape    = RoundedCornerShape(10.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(Modifier.padding(12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.Default.Error, null, tint = NeonRed)
                        Text(s.message, color = NeonRed, fontSize = 13.sp)
                    }
                }
                TextButton(onClick = { viewModel.download(context) }) {
                    Text("Retry", color = NeonBlue)
                }
            }
        }

        // GGUF info
        HorizontalDivider(color = BorderDim, modifier = Modifier.padding(top = 8.dp))
        Text("TECHNICAL NOTES", color = TextDim, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
        InfoRow("Format",    "GGUF (llama.cpp compatible)")
        InfoRow("Runtime",   "ONNX Runtime / llama.cpp JNI bridge (planned)")
        InfoRow("Privacy",   "Model stored in app-private dir, never synced")
        InfoRow("Indexer",   "Used by Meme Indexer to tag images semantically")
        InfoRow("Chat",      "Drives multimodal chat streaming responses")
    }
}

// ── Sub-composables ────────────────────────────────────────────────────────────

@Composable
private fun ModelTierCard(
    tier:      ModelTier,
    selected:  Boolean,
    available: Boolean,
    onSelect:  () -> Unit,
    onDelete:  () -> Unit,
) {
    Card(
        onClick  = onSelect,
        colors   = CardDefaults.cardColors(containerColor = if (selected) NeonBlue.copy(0.08f) else BgCard),
        shape    = RoundedCornerShape(10.dp),
        border   = BorderStroke(1.dp, if (selected) NeonBlue.copy(0.5f) else BorderDim),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment     = Alignment.CenterVertically,
        ) {
            RadioButton(
                selected = selected,
                onClick  = onSelect,
                colors   = RadioButtonDefaults.colors(selectedColor = NeonBlue),
            )
            Column(Modifier.weight(1f)) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically) {
                    Text(tier.displayName, color = if (selected) NeonBlue else TextPrimary,
                        fontSize = 13.sp, fontWeight = FontWeight.Medium)
                    if (available) {
                        Box(
                            Modifier
                                .background(NeonGreen.copy(0.15f), RoundedCornerShape(4.dp))
                                .border(0.5.dp, NeonGreen.copy(0.4f), RoundedCornerShape(4.dp))
                                .padding(horizontal = 5.dp, vertical = 1.dp)
                        ) { Text("READY", fontSize = 9.sp, color = NeonGreen, fontWeight = FontWeight.Bold,
                            fontFamily = FontFamily.Monospace) }
                    }
                }
                Text(
                    "~${tier.sizeMb} MB  ·  ${tier.paramBillions}B params  ·  ${tier.minRamMb} MB RAM min",
                    color = TextMuted, fontSize = 11.sp,
                )
            }
            if (available) {
                IconButton(onClick = onDelete, modifier = Modifier.size(32.dp)) {
                    Icon(Icons.Default.Delete, null, tint = NeonRed, modifier = Modifier.size(18.dp))
                }
            }
        }
    }
}

@Composable
private fun ProgressSection(label: String, progress: Float, color: androidx.compose.ui.graphics.Color) {
    val anim by animateFloatAsState(progress, tween(300), label = "dl")
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
            Text(label, color = color, fontSize = 13.sp)
            Text("${(progress * 100).toInt()}%", color = color, fontSize = 13.sp,
                fontFamily = FontFamily.Monospace)
        }
        LinearProgressIndicator(
            progress   = { anim },
            modifier   = Modifier.fillMaxWidth().height(6.dp),
            color      = color,
            trackColor = BorderDim,
        )
    }
}

@Composable
private fun ReadyBadge(tier: ModelTier) {
    Card(
        colors   = CardDefaults.cardColors(containerColor = NeonGreen.copy(0.08f)),
        shape    = RoundedCornerShape(10.dp),
        border   = BorderStroke(1.dp, NeonGreen.copy(0.4f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(14.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment     = Alignment.CenterVertically,
        ) {
            Icon(Icons.Default.CheckCircle, null, tint = NeonGreen, modifier = Modifier.size(24.dp))
            Column {
                Text("${tier.displayName} ready", color = NeonGreen, fontWeight = FontWeight.Bold)
                Text("On-device inference active — go to Chat or Memes to use it.",
                    color = TextMuted, fontSize = 12.sp)
            }
        }
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp),
        horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, color = TextDim, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
        Text(value, color = TextMuted, fontSize = 11.sp)
    }
}
