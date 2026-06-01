package com.example.qualia.memes

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.staggeredgrid.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import com.example.qualia.theme.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MemeScreen(viewModel: MemeViewModel) {
    val context = LocalContext.current
    val memes   by viewModel.memes.collectAsState()
    val query   by viewModel.searchQuery.collectAsState()
    val results by viewModel.searchResults.collectAsState()

    // Gallery picker
    val launcher = rememberLauncherForActivityResult(
        ActivityResultContracts.GetMultipleContents()
    ) { uris: List<Uri> ->
        uris.forEach { viewModel.importImage(context, it) }
    }

    Scaffold(
        containerColor = BgDeep,
        floatingActionButton = {
            FloatingActionButton(
                onClick            = { launcher.launch("image/*") },
                containerColor     = NeonBlue,
                contentColor       = BgDeep,
            ) { Icon(Icons.Default.Add, contentDescription = "Import memes") }
        },
        topBar = {
            Column(
                Modifier
                    .background(BgDeep)
                    .padding(horizontal = 16.dp, vertical = 8.dp)
            ) {
                Text(
                    "MEME LIBRARY",
                    style = MaterialTheme.typography.displayLarge.copy(fontSize = 18.sp),
                    modifier = Modifier.padding(bottom = 8.dp),
                )
                OutlinedTextField(
                    value         = query,
                    onValueChange = viewModel::setSearchQuery,
                    placeholder   = { Text("Search by topic, emotion, caption…", color = TextMuted) },
                    leadingIcon   = { Icon(Icons.Default.Search, contentDescription = null, tint = NeonBlue) },
                    modifier      = Modifier.fillMaxWidth(),
                    shape         = RoundedCornerShape(8.dp),
                    colors        = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor   = NeonBlue,
                        unfocusedBorderColor = BorderDim,
                        focusedTextColor     = TextPrimary,
                        unfocusedTextColor   = TextPrimary,
                        cursorColor          = NeonBlue,
                    ),
                    singleLine    = true,
                )
                // Stats bar
                val total     = memes.size
                val indexed   = memes.count { it.indexed }
                val unindexed = total - indexed
                Row(
                    Modifier.padding(top = 6.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    StatChip("$total memes",     NeonBlue)
                    StatChip("$indexed indexed", NeonGreen)
                    if (unindexed > 0)
                        StatChip("$unindexed pending", NeonGold)
                }
            }
        }
    ) { innerPadding ->
        if (results.isEmpty() && query.isNotBlank()) {
            EmptySearchState(query, Modifier.padding(innerPadding))
        } else {
            val displayList = if (query.isBlank()) memes else results.map { it.first }
            LazyVerticalStaggeredGrid(
                columns               = StaggeredGridCells.Adaptive(140.dp),
                modifier              = Modifier.padding(innerPadding),
                contentPadding        = PaddingValues(8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalItemSpacing   = 8.dp,
            ) {
                items(displayList, key = { it.id }) { meme ->
                    MemeCard(meme, onClick = { viewModel.selectMeme(meme) })
                }
            }
        }
    }

    // Detail bottom sheet
    viewModel.selectedMeme.collectAsState().value?.let { meme ->
        MemeDetailSheet(meme, onDismiss = { viewModel.selectMeme(null) }, onDelete = { viewModel.delete(it) })
    }
}

@Composable
private fun MemeCard(meme: MemeEntry, onClick: () -> Unit) {
    Box(
        Modifier
            .clip(RoundedCornerShape(8.dp))
            .background(BgCard)
            .border(1.dp, if (meme.indexed) BorderGlow else BorderDim, RoundedCornerShape(8.dp))
            .clickable(onClick = onClick)
    ) {
        Column {
            AsyncImage(
                model             = meme.fileUri,
                contentDescription = meme.caption ?: "meme",
                contentScale      = ContentScale.Crop,
                modifier          = Modifier.fillMaxWidth().heightIn(80.dp, 200.dp),
            )
            if (meme.indexed) {
                Column(Modifier.padding(6.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    meme.topic?.let   { SemanticChip(it, NeonBlue) }
                    meme.emotion?.let { SemanticChip(it, NeonPurple) }
                    meme.useWhen.take(1).forEach { SemanticChip(it, NeonGold) }
                }
            } else {
                Text(
                    "Not yet indexed",
                    color    = TextDim,
                    fontSize = 10.sp,
                    modifier = Modifier.padding(6.dp),
                )
            }
        }
    }
}

@Composable
private fun SemanticChip(text: String, color: androidx.compose.ui.graphics.Color) {
    Box(
        Modifier
            .background(color.copy(alpha = 0.15f), RoundedCornerShape(4.dp))
            .border(0.5.dp, color.copy(alpha = 0.4f), RoundedCornerShape(4.dp))
            .padding(horizontal = 5.dp, vertical = 2.dp)
    ) {
        Text(text, fontSize = 10.sp, color = color, fontWeight = FontWeight.Medium)
    }
}

@Composable
private fun StatChip(label: String, color: androidx.compose.ui.graphics.Color) {
    Text(label, fontSize = 11.sp, color = color, fontWeight = FontWeight.SemiBold)
}

@Composable
private fun EmptySearchState(query: String, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("🔍", fontSize = 40.sp)
            Spacer(Modifier.height(12.dp))
            Text("No memes found for", color = TextMuted, fontSize = 14.sp)
            Text("\"$query\"", color = NeonBlue, fontSize = 16.sp, fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(8.dp))
            Text("Try different keywords, or import more memes.", color = TextDim, fontSize = 12.sp)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MemeDetailSheet(meme: MemeEntry, onDismiss: () -> Unit, onDelete: (String) -> Unit) {
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor   = BgCard,
    ) {
        Column(Modifier.padding(16.dp).navigationBarsPadding()) {
            AsyncImage(
                model             = meme.fileUri,
                contentDescription = meme.caption,
                modifier          = Modifier.fillMaxWidth().heightIn(120.dp, 300.dp).clip(RoundedCornerShape(8.dp)),
                contentScale      = ContentScale.Fit,
            )
            Spacer(Modifier.height(12.dp))

            if (meme.indexed) {
                LabelRow("Topic",    meme.topic ?: "-")
                LabelRow("Emotion",  meme.emotion ?: "-")
                meme.caption?.let { LabelRow("Caption", it) }
                if (meme.useWhen.isNotEmpty()) {
                    Text("Use when:", color = TextMuted, fontSize = 12.sp, modifier = Modifier.padding(top = 8.dp))
                    meme.useWhen.forEach { Text("• $it", color = TextPrimary, fontSize = 13.sp) }
                }
                Text(
                    "Confidence: ${"%.0f".format(meme.confidence * 100)}%",
                    color = NeonGreen, fontSize = 11.sp, modifier = Modifier.padding(top = 6.dp)
                )
            } else {
                Text("Not yet indexed — open the LLM to analyse this meme.", color = TextMuted)
            }

            Spacer(Modifier.height(16.dp))
            OutlinedButton(
                onClick = { onDelete(meme.id); onDismiss() },
                colors  = ButtonDefaults.outlinedButtonColors(contentColor = NeonRed),
                border  = ButtonDefaults.outlinedButtonBorder.copy(),
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Delete from library") }
        }
    }
}

@Composable
private fun LabelRow(label: String, value: String) {
    Row(Modifier.padding(vertical = 3.dp)) {
        Text("$label: ", color = TextMuted, fontSize = 13.sp, fontWeight = FontWeight.Medium)
        Text(value,       color = TextPrimary, fontSize = 13.sp)
    }
}
