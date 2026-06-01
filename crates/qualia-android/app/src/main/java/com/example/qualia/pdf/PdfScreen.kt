package com.example.qualia.pdf

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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

class PdfViewModel : ViewModel() {
    private val _pages   = MutableStateFlow<List<PageResult>>(emptyList())
    val pages: StateFlow<List<PageResult>> = _pages

    private val _result = MutableStateFlow<DatasetResult?>(null)
    val result: StateFlow<DatasetResult?> = _result

    private val _scanning = MutableStateFlow(false)
    val scanning: StateFlow<Boolean> = _scanning

    private val _progress = MutableStateFlow(0 to 0)      // scanned to total
    val progress: StateFlow<Pair<Int,Int>> = _progress

    fun scan(context: android.content.Context, uri: Uri) {
        viewModelScope.launch {
            _scanning.value = true
            _pages.value    = emptyList()
            _result.value   = null

            val scanner = PdfScanner(context)
            // Stream pages to UI in real time
            scanner.scanPages(uri).collect { page ->
                _pages.value = _pages.value + page
            }

            // Build the full N-Quad dataset
            val ds = scanner.buildDataset(uri) { scanned, total ->
                _progress.value = scanned to total
            }
            _result.value   = ds
            _scanning.value = false
        }
    }

    fun reset() {
        _pages.value   = emptyList()
        _result.value  = null
        _progress.value = 0 to 0
    }
}

// ── Screen ────────────────────────────────────────────────────────────────────

@Composable
fun PdfScreen(viewModel: PdfViewModel) {
    val context  = LocalContext.current
    val pages    by viewModel.pages.collectAsState()
    val result   by viewModel.result.collectAsState()
    val scanning by viewModel.scanning.collectAsState()
    val progress by viewModel.progress.collectAsState()

    var pickedUri  by remember { mutableStateOf<Uri?>(null) }
    var pickedName by remember { mutableStateOf("") }

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
    ) {
        // ── Header ─────────────────────────────────────────────────────────────
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text("PDF → DATASET", style = MaterialTheme.typography.displayLarge.copy(fontSize = 18.sp))
            Text("Scan PDF pages into N-Quad semantic datasets",
                color = TextMuted, fontSize = 13.sp)

            // File picker
            Card(
                onClick  = { picker.launch("application/pdf") },
                colors   = CardDefaults.cardColors(containerColor = BgCard),
                shape    = RoundedCornerShape(10.dp),
                border   = BorderStroke(1.dp, if (pickedUri != null) NeonBlue.copy(0.5f) else BorderDim),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    Modifier.padding(14.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalAlignment     = Alignment.CenterVertically,
                ) {
                    Icon(Icons.Default.PictureAsPdf, null, tint = NeonRed, modifier = Modifier.size(26.dp))
                    Column(Modifier.weight(1f)) {
                        Text(if (pickedUri != null) pickedName else "Tap to pick a PDF",
                            color = if (pickedUri != null) TextPrimary else TextMuted, fontSize = 13.sp)
                        if (pickedUri != null) Text("Tap to change", color = TextDim, fontSize = 11.sp)
                    }
                    if (pickedUri != null) Icon(Icons.Default.CheckCircle, null, tint = NeonGreen)
                }
            }

            // Scan button
            Button(
                onClick  = { pickedUri?.let { viewModel.scan(context, it) } },
                enabled  = pickedUri != null && !scanning,
                colors   = ButtonDefaults.buttonColors(
                    containerColor = NeonRed, contentColor = BgDeep,
                    disabledContainerColor = BorderDim,
                ),
                modifier = Modifier.fillMaxWidth(),
                shape    = RoundedCornerShape(8.dp),
            ) {
                Icon(Icons.Default.DocumentScanner, null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text("Scan PDF", fontWeight = FontWeight.Bold, fontSize = 15.sp)
            }

            // Progress / result banner
            if (scanning) {
                val (done, total) = progress
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Scanning page ${done} of ${total.takeIf { it > 0 } ?: "?"}…",
                        color = NeonRed, fontSize = 13.sp)
                    LinearProgressIndicator(
                        progress   = { if (total > 0) done.toFloat() / total else 0f },
                        modifier   = Modifier.fillMaxWidth().height(4.dp),
                        color      = NeonRed,
                        trackColor = BorderDim,
                    )
                }
            }

            result?.let { ds ->
                DatasetBanner(ds)
            }
        }

        HorizontalDivider(color = BorderDim)

        // ── Page list ──────────────────────────────────────────────────────────
        if (pages.isEmpty() && !scanning) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("📄", fontSize = 40.sp)
                    Text("Pick a PDF and tap Scan.", color = TextMuted, fontSize = 14.sp)
                    Text("Each page becomes a set of N-Quads.", color = TextDim, fontSize = 12.sp)
                }
            }
        } else {
            LazyColumn(
                contentPadding = PaddingValues(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(pages, key = { it.pageIndex }) { page ->
                    PageCard(page)
                }
            }
        }
    }
}

@Composable
private fun PageCard(page: PageResult) {
    Card(
        colors   = CardDefaults.cardColors(containerColor = BgCard),
        shape    = RoundedCornerShape(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                Text("Page ${page.pageIndex + 1}", color = NeonRed, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                Text(page.quadUri.takeLast(20), color = TextDim, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
            }
            val displayText = page.text.take(300).ifBlank { "No text layer — OCR pending" }
            Text(displayText, color = if (page.text.isBlank()) TextDim else TextMuted, fontSize = 12.sp, maxLines = 6)
        }
    }
}

@Composable
private fun DatasetBanner(ds: DatasetResult) {
    Card(
        colors   = CardDefaults.cardColors(containerColor = NeonGreen.copy(0.08f)),
        shape    = RoundedCornerShape(10.dp),
        border   = BorderStroke(1.dp, NeonGreen.copy(0.4f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment     = Alignment.CenterVertically,
        ) {
            Icon(Icons.Default.CheckCircle, null, tint = NeonGreen)
            Column {
                Text("Dataset ready", color = NeonGreen, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                Text("${ds.nquads.size} N-Quads · ${ds.pageCount} pages · ${ds.outputPath.substringAfterLast("/")}",
                    color = TextMuted, fontSize = 11.sp)
            }
        }
    }
}
