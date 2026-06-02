package com.example.qualia

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import com.example.qualia.chat.ChatScreen
import com.example.qualia.chat.ChatViewModel
import com.example.qualia.demos.DemoScreen
import com.example.qualia.llm.LlmScreen
import com.example.qualia.llm.LlmViewModel
import com.example.qualia.memes.MemeScreen
import com.example.qualia.memes.MemeViewModel
import com.example.qualia.ontology.OntologyScreen
import com.example.qualia.ontology.OntologyViewModel
import com.example.qualia.pdf.PdfScreen
import com.example.qualia.pdf.PdfViewModel
import com.example.qualia.pfm.PfmFlow
import com.example.qualia.theme.*

// ── Nav destinations ──────────────────────────────────────────────────────────

sealed class NavDest(val route: String, val label: String, val icon: ImageVector) {
    object Chat     : NavDest("chat",     "Chat",    Icons.Default.Chat)
    object Memes    : NavDest("memes",    "Memes",   Icons.Default.EmojiEmotions)
    object PDF      : NavDest("pdf",      "PDF",     Icons.Default.PictureAsPdf)
    object Ontology : NavDest("ontology", "Onto",    Icons.Default.Hub)
    object LLM      : NavDest("llm",      "LLM",     Icons.Default.Psychology)
    object Demos    : NavDest("demos",    "Demos",   Icons.Default.PlayArrow)
    object Ledger   : NavDest("ledger",   "Ledger",  Icons.Default.AccountBalanceWallet)
    object Settings : NavDest("settings", "Settings",Icons.Default.Settings)
}

private val navItems = listOf(
    NavDest.Chat,
    NavDest.Memes,
    NavDest.PDF,
    NavDest.Ontology,
    NavDest.LLM,
    NavDest.Demos,
    NavDest.Ledger,
    NavDest.Settings,
)

// ── Navigation host ───────────────────────────────────────────────────────────

@Composable
fun MainNavigation(
    chatViewModel:     ChatViewModel,
    memeViewModel:     MemeViewModel,
    pdfViewModel:      PdfViewModel,
    ontologyViewModel: OntologyViewModel,
    llmViewModel:      LlmViewModel,
) {
    var current by remember { mutableStateOf<NavDest>(NavDest.Chat) }

    Scaffold(
        containerColor = BgDeep,
        bottomBar = {
            NavigationBar(containerColor = BgCard, tonalElevation = 0.dp) {
                navItems.forEach { dest ->
                    val selected = current == dest
                    NavigationBarItem(
                        selected = selected,
                        onClick  = { current = dest },
                        icon     = {
                            Icon(
                                dest.icon,
                                contentDescription = dest.label,
                                tint = if (selected) NeonBlue else TextMuted,
                            )
                        },
                        label    = {
                            Text(
                                dest.label,
                                color = if (selected) NeonBlue else TextMuted,
                                style = MaterialTheme.typography.labelSmall,
                            )
                        },
                        colors   = NavigationBarItemDefaults.colors(
                            indicatorColor = NeonBlue.copy(alpha = 0.15f),
                        ),
                    )
                }
            }
        },
    ) { innerPadding ->
        val contentMod = Modifier.padding(innerPadding)
        when (current) {
            NavDest.Chat     -> ChatScreen(chatViewModel, memeViewModel)
            NavDest.Memes    -> MemeScreen(memeViewModel)
            NavDest.PDF      -> PdfScreen(pdfViewModel)
            NavDest.Ontology -> OntologyScreen(ontologyViewModel)
            NavDest.LLM      -> LlmScreen(llmViewModel)
            NavDest.Demos    -> DemoScreen()
            NavDest.Ledger   -> PfmFlow()
            NavDest.Settings -> SettingsPlaceholder(contentMod)
            else -> {}
        }
    }
}

@Composable
private fun SettingsPlaceholder(modifier: Modifier = Modifier) {
    Surface(modifier.fillMaxSize(), color = BgDeep) {
        Box(contentAlignment = Alignment.Center) {
            Text("Settings — tax suite, ILP addresses, update prefs", color = TextMuted)
        }
    }
}
