package com.example.qualia.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import com.example.qualia.theme.BgDeep
import com.example.qualia.theme.NeonBlue
import com.example.qualia.theme.TextMuted

data class VaultSection(val title: String, val icon: ImageVector)

val vaultSections = listOf(
    VaultSection("Personal Health", Icons.Default.Favorite),
    VaultSection("Timeline", Icons.Default.Timeline),
    VaultSection("Pathology & Labs", Icons.Default.Science),
    VaultSection("Case Management", Icons.Default.FolderShared),
    VaultSection("Psychiatry", Icons.Default.Psychology),
    VaultSection("Documents", Icons.Default.Description),
    VaultSection("Study Vault", Icons.Default.School),
    VaultSection("Agent Directory", Icons.Default.Contacts),
    VaultSection("Vault Admin", Icons.Default.AdminPanelSettings)
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultDashboardScreen(
    isSanctuaryLaneActive: Boolean,
    onNavigateToSection: (String) -> Unit
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { 
                    Text(if (isSanctuaryLaneActive) "Sanctuary Vault" else "Standard Vault") 
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = if (isSanctuaryLaneActive) MaterialTheme.colorScheme.errorContainer else BgDeep
                ),
                actions = {
                    if (isSanctuaryLaneActive) {
                        Icon(Icons.Default.Security, contentDescription = "Sanctuary Active", tint = NeonBlue)
                    }
                }
            )
        }
    ) { padding ->
        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
            modifier = Modifier.fillMaxSize().background(BgDeep).padding(padding)
        ) {
            items(vaultSections) { section ->
                Card(
                    onClick = { onNavigateToSection(section.title) },
                    modifier = Modifier.height(120.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
                ) {
                    Column(
                        modifier = Modifier.fillMaxSize().padding(16.dp),
                        verticalArrangement = Arrangement.Center,
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Icon(
                            imageVector = section.icon, 
                            contentDescription = section.title,
                            tint = NeonBlue,
                            modifier = Modifier.size(32.dp)
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = section.title, 
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                            textAlign = androidx.compose.ui.text.style.TextAlign.Center
                        )
                    }
                }
            }
        }
    }
}
