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
import com.example.qualia.memes.MemeScreen
import com.example.qualia.memes.MemeViewModel
import com.example.qualia.theme.*

// ── Nav destinations ──────────────────────────────────────────────────────────

sealed class NavDest(val route: String, val label: String, val icon: ImageVector) {
    object Chat     : NavDest("chat",     "Chat",     Icons.Default.Chat)
    object Memes    : NavDest("memes",    "Memes",    Icons.Default.EmojiEmotions)
    object Demos    : NavDest("demos",    "Demos",    Icons.Default.PlayArrow)
    object Settings : NavDest("settings", "Settings", Icons.Default.Settings)
}

private val navItems = listOf(NavDest.Chat, NavDest.Memes, NavDest.Demos, NavDest.Settings)

// ── Main navigation host ──────────────────────────────────────────────────────

@Composable
fun MainNavigation(
    chatViewModel: ChatViewModel,
    memeViewModel: MemeViewModel,
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
                            Icon(dest.icon, contentDescription = dest.label,
                                tint = if (selected) NeonBlue else TextMuted)
                        },
                        label    = {
                            Text(dest.label,
                                color = if (selected) NeonBlue else TextMuted,
                                style = MaterialTheme.typography.labelSmall)
                        },
                        colors   = NavigationBarItemDefaults.colors(
                            indicatorColor = NeonBlue.copy(alpha = 0.15f),
                        ),
                    )
                }
            }
        },
    ) { innerPadding ->
        when (current) {
            NavDest.Chat  -> ChatScreen(chatViewModel, memeViewModel)
            NavDest.Memes -> MemeScreen(memeViewModel)
            NavDest.Demos -> DemoScreen()
            NavDest.Settings -> SettingsPlaceholder(Modifier.padding(innerPadding))
            else -> {}
        }
    }
}

@Composable
private fun SettingsPlaceholder(modifier: Modifier = Modifier) {
    Surface(modifier.fillMaxSize(), color = BgDeep) {
        Box(contentAlignment = Alignment.Center) {
            Text("Settings — coming next iteration", color = TextMuted)
        }
    }
}
