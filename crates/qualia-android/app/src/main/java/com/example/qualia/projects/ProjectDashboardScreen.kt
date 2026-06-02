package com.example.qualia.projects

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.qualia.ui.components.PremiumButton

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProjectDashboardScreen(
    onNavigateToCommit: () -> Unit,
    onNavigateBack: () -> Unit
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Cooperative Projects") },
                navigationIcon = {
                    PremiumButton(
                        text = "< Back", 
                        onClick = onNavigateBack,
                        modifier = Modifier.padding(start = 8.dp)
                    )
                },
                actions = {
                    IconButton(onClick = { /* Trigger P2P Sync */ }) {
                        Icon(Icons.Default.Sync, contentDescription = "Sync Project")
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(onClick = onNavigateToCommit) {
                Icon(Icons.Default.Add, contentDescription = "New Commit")
            }
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            Text(
                "Obligation Matrix", 
                fontSize = 20.sp, 
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(bottom = 16.dp)
            )

            // Mock Data for the prototype
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                item {
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp)) {
                            Text("Alice (did:key:z6Mk...)", fontWeight = FontWeight.Bold)
                            Text("Financial Contribution: $1,200.00")
                            Text("Labor Obligation: 45 Hours")
                        }
                    }
                }
                item {
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp)) {
                            Text("Bob (did:key:z6Mj...)", fontWeight = FontWeight.Bold)
                            Text("Financial Contribution: $300.00")
                            Text("Labor Obligation: 12 Hours")
                        }
                    }
                }
            }
        }
    }
}
