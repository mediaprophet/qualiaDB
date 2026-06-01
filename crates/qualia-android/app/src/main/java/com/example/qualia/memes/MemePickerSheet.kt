package com.example.qualia.memes

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.example.qualia.theme.*

/**
 * MemePickerSheet — modal bottom sheet used by the Chat screen.
 *
 * Opened when the user taps the 🖼️ meme button, or when the LLM tool suggests a meme.
 * Pre-fills the search query with the current chat topic so results are immediately relevant.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MemePickerSheet(
    memeViewModel: MemeViewModel,
    prefilledQuery: String = "",
    onMemeSelected: (MemeEntry) -> Unit,
    onDismiss: () -> Unit,
) {
    var query by remember { mutableStateOf(prefilledQuery) }
    val allMemes by memeViewModel.memes.collectAsState()
    val results = remember(query, allMemes) {
        if (query.isBlank()) allMemes.map { it to 0 }
        else memeViewModel.getLibrary()?.search(query) ?: emptyList()
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor   = BgCard,
        dragHandle       = {
            Box(
                Modifier
                    .padding(vertical = 8.dp)
                    .size(width = 40.dp, height = 4.dp)
                    .background(BorderDim, RoundedCornerShape(2.dp))
            )
        },
    ) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = 16.dp)
        ) {
            Text(
                "FIND A MEME",
                style    = MaterialTheme.typography.displayLarge.copy(fontSize = 15.sp),
                modifier = Modifier.padding(bottom = 12.dp),
            )

            OutlinedTextField(
                value         = query,
                onValueChange = { query = it },
                placeholder   = { Text("What's the vibe? (e.g. 'shocked', 'Monday'…)", color = TextMuted, fontSize = 13.sp) },
                leadingIcon   = { Icon(Icons.Default.Search, null, tint = NeonBlue) },
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

            Spacer(Modifier.height(8.dp))

            if (results.isEmpty()) {
                Box(
                    Modifier.fillMaxWidth().height(200.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("😶", fontSize = 36.sp)
                        Spacer(Modifier.height(8.dp))
                        Text("No memes in your library yet.", color = TextMuted, fontSize = 13.sp)
                        Text("Import some from the Meme Library tab.", color = TextDim, fontSize = 12.sp)
                    }
                }
            } else {
                // Compact result score bar for semantic results
                if (query.isNotBlank()) {
                    Text(
                        "${results.size} result${if (results.size != 1) "s" else ""} · ranked by relevance",
                        color = TextMuted, fontSize = 11.sp,
                        modifier = Modifier.padding(bottom = 6.dp),
                    )
                }

                LazyVerticalGrid(
                    columns               = GridCells.Adaptive(100.dp),
                    modifier              = Modifier.heightIn(max = 420.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalArrangement   = Arrangement.spacedBy(6.dp),
                    contentPadding        = PaddingValues(bottom = 16.dp),
                ) {
                    items(results.size) { i ->
                        val (meme, hits) = results[i]
                        PickerMemeCard(meme, hits, onSelect = { onMemeSelected(meme) })
                    }
                }
            }
        }
    }
}

@Composable
private fun PickerMemeCard(meme: MemeEntry, hits: Int, onSelect: () -> Unit) {
    Box(
        Modifier
            .clip(RoundedCornerShape(8.dp))
            .background(BgDeep)
            .border(
                1.dp,
                if (hits > 1) NeonBlue.copy(alpha = 0.5f) else BorderDim,
                RoundedCornerShape(8.dp)
            )
            .clickable(onClick = onSelect)
    ) {
        Column {
            AsyncImage(
                model             = meme.fileUri,
                contentDescription = meme.caption ?: "meme",
                modifier          = Modifier.fillMaxWidth().height(90.dp),
                contentScale      = ContentScale.Crop,
            )
            // Relevance bar
            if (hits > 0) {
                Box(
                    Modifier
                        .fillMaxWidth((hits.coerceAtMost(5) / 5f))
                        .height(2.dp)
                        .background(NeonGold),
                )
            }
            meme.topic?.let {
                Text(it, fontSize = 9.sp, color = NeonBlue,
                    modifier = Modifier.padding(horizontal = 5.dp, vertical = 3.dp),
                    maxLines = 1,
                )
            }
        }
    }
}
