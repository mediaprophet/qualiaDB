package com.example.qualia.ontology

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.core.*
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
import androidx.compose.ui.draw.clip
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

class OntologyViewModel : ViewModel() {
    private val _state = MutableStateFlow<OntologyUiState>(OntologyUiState.Idle)
    val state: StateFlow<OntologyUiState> = _state

    fun convert(context: android.content.Context, uri: Uri, format: OntologyFormat, outputFormat: OutputFormat) {
        viewModelScope.launch {
            _state.value = OntologyUiState.Converting(0f)
            val converter = OntologyConverter(context)
            runCatching {
                val result = converter.convert(uri, format, outputFormat = outputFormat) { p ->
                    _state.value = OntologyUiState.Converting(p)
                }
                _state.value = OntologyUiState.Done(result)
            }.onFailure { e ->
                _state.value = OntologyUiState.Error(e.message ?: "Conversion failed")
            }
        }
    }

    fun reset() { _state.value = OntologyUiState.Idle }
}

sealed class OntologyUiState {
    object Idle                                          : OntologyUiState()
    data class Converting(val progress: Float)           : OntologyUiState()
    data class Done(val result: ConversionResult)        : OntologyUiState()
    data class Error(val message: String)                : OntologyUiState()
}

// ── Screen ────────────────────────────────────────────────────────────────────

@Composable
fun OntologyScreen(viewModel: OntologyViewModel) {
    val context  = LocalContext.current
    val state    by viewModel.state.collectAsState()
    var pickedUri   by remember { mutableStateOf<Uri?>(null) }
    var pickedName  by remember { mutableStateOf("") }
    var selectedFmt by remember { mutableStateOf(OntologyFormat.N_TRIPLES) }
    var outputFmt   by remember { mutableStateOf(OutputFormat.N_QUADS) }

    val picker = rememberLauncherForActivityResult(
        ActivityResultContracts.GetContent()
    ) { uri ->
        pickedUri  = uri
        pickedName = uri?.lastPathSegment ?: ""
        viewModel.reset()
    }

    Column(
        Modifier
            .fillMaxSize()
            .background(BgDeep)
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        // Header
        Text("ONTOLOGY CONVERTER", style = MaterialTheme.typography.displayLarge.copy(fontSize = 18.sp))
        Text("Convert RDF, Turtle, JSON-LD, or CSV → N-Quads / .q42 ledger",
            color = TextMuted, fontSize = 13.sp)

        // File picker card
        Card(
            onClick    = { picker.launch("*/*") },
            colors     = CardDefaults.cardColors(containerColor = BgCard),
            shape      = RoundedCornerShape(12.dp),
            border     = BorderStroke(1.dp, if (pickedUri != null) NeonBlue.copy(0.5f) else BorderDim),
            modifier   = Modifier.fillMaxWidth(),
        ) {
            Row(
                Modifier.padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment     = Alignment.CenterVertically,
            ) {
                Icon(Icons.Default.FolderOpen, contentDescription = null,
                    tint = NeonBlue, modifier = Modifier.size(28.dp))
                Column(Modifier.weight(1f)) {
                    Text(if (pickedUri != null) "File selected" else "Tap to pick a file",
                        color = if (pickedUri != null) NeonBlue else TextMuted, fontSize = 14.sp)
                    if (pickedName.isNotBlank())
                        Text(pickedName, color = TextPrimary, fontSize = 12.sp, maxLines = 1)
                }
                if (pickedUri != null)
                    Icon(Icons.Default.CheckCircle, null, tint = NeonGreen)
            }
        }

        // Format selector
        Text("Input format", color = TextMuted, fontSize = 13.sp, fontWeight = FontWeight.Medium)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OntologyFormat.entries.forEach { fmt ->
                FilterChip(
                    selected = selectedFmt == fmt,
                    onClick  = { selectedFmt = fmt },
                    label    = { Text(fmt.label, fontSize = 11.sp) },
                    colors   = FilterChipDefaults.filterChipColors(
                        selectedContainerColor = NeonBlue.copy(0.2f),
                        selectedLabelColor     = NeonBlue,
                    ),
                )
            }
        }

        // Output format
        Text("Output format", color = TextMuted, fontSize = 13.sp, fontWeight = FontWeight.Medium)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutputFormat.entries.forEach { fmt ->
                val label = when (fmt) {
                    OutputFormat.N_QUADS    -> "N-Quads (.nq)"
                    OutputFormat.Q42_BINARY -> ".q42 Binary Ledger"
                }
                FilterChip(
                    selected = outputFmt == fmt,
                    onClick  = { outputFmt = fmt },
                    label    = { Text(label, fontSize = 11.sp) },
                    colors   = FilterChipDefaults.filterChipColors(
                        selectedContainerColor = NeonPurple.copy(0.2f),
                        selectedLabelColor     = NeonPurple,
                    ),
                )
            }
        }

        // Convert button
        Button(
            onClick  = { pickedUri?.let { viewModel.convert(context, it, selectedFmt, outputFmt) } },
            enabled  = pickedUri != null && state !is OntologyUiState.Converting,
            colors   = ButtonDefaults.buttonColors(
                containerColor = NeonBlue, contentColor = BgDeep,
                disabledContainerColor = BorderDim,
            ),
            modifier = Modifier.fillMaxWidth(),
            shape    = RoundedCornerShape(8.dp),
        ) {
            Icon(Icons.Default.Transform, null, modifier = Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text("Convert", fontWeight = FontWeight.Bold, fontSize = 15.sp)
        }

        // State display
        when (val s = state) {
            is OntologyUiState.Converting -> ConvertingIndicator(s.progress)
            is OntologyUiState.Done       -> ConversionResultCard(s.result)
            is OntologyUiState.Error      -> ErrorCard(s.message)
            else -> {}
        }
    }
}

// ── Sub-composables ────────────────────────────────────────────────────────────

@Composable
private fun ConvertingIndicator(progress: Float) {
    val anim by animateFloatAsState(progress, tween(300), label = "progress")
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("Converting…", color = NeonBlue, fontSize = 13.sp)
        LinearProgressIndicator(
            progress   = { anim },
            modifier   = Modifier.fillMaxWidth().height(6.dp),
            color      = NeonBlue,
            trackColor = BorderDim,
        )
        Text("${(progress * 100).toInt()}%", color = TextMuted, fontSize = 11.sp,
            fontFamily = FontFamily.Monospace)
    }
}

@Composable
private fun ConversionResultCard(result: ConversionResult) {
    Card(
        colors = CardDefaults.cardColors(containerColor = NeonGreen.copy(0.08f)),
        shape  = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, NeonGreen.copy(0.4f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Icon(Icons.Default.CheckCircle, null, tint = NeonGreen)
                Text("Conversion complete", color = NeonGreen, fontWeight = FontWeight.Bold)
            }
            ResultRow("Quads written",  result.quadCount.toString())
            ResultRow("Output format",  result.format)
            ResultRow("Duration",       "${result.durationMs} ms")
            ResultRow("File",           result.outputPath.substringAfterLast("/"))

            if (result.warnings.isNotEmpty()) {
                result.warnings.forEach { w ->
                    Text("⚠ $w", color = NeonGold, fontSize = 11.sp)
                }
            }

            OutlinedButton(
                onClick  = { /* TODO: Android ShareSheet */ },
                colors   = ButtonDefaults.outlinedButtonColors(contentColor = NeonBlue),
                modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
                shape    = RoundedCornerShape(8.dp),
            ) {
                Icon(Icons.Default.Share, null, modifier = Modifier.size(16.dp))
                Spacer(Modifier.width(6.dp))
                Text("Share / Export")
            }
        }
    }
}

@Composable
private fun ErrorCard(message: String) {
    Card(
        colors = CardDefaults.cardColors(containerColor = NeonRed.copy(0.08f)),
        shape  = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, NeonRed.copy(0.4f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(14.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Icon(Icons.Default.Error, null, tint = NeonRed)
            Text(message, color = NeonRed, fontSize = 13.sp)
        }
    }
}

@Composable
private fun ResultRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, color = TextMuted, fontSize = 12.sp)
        Text(value, color = TextPrimary, fontSize = 12.sp, fontWeight = FontWeight.Medium,
            fontFamily = FontFamily.Monospace)
    }
}
