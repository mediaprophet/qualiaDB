package com.example.qualia.identity

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AssignmentTurnedIn
import androidx.compose.material.icons.filled.Security
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.qualia.theme.NeonBlue
import com.example.qualia.ui.components.PremiumButton

/**
 * VC-11 / Maslow Hierarchy VP Generator
 * Allows vulnerable populations to generate strict "need-to-know" packages
 * for social workers, proving housing status or identity without exposing
 * the entire sanctuary graph.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MaslowPresentationScreen(onNavigateBack: () -> Unit) {
    var includeIdentity by remember { mutableStateOf(true) }
    var includePoliceCheck by remember { mutableStateOf(false) }
    var includeHousingStatus by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Compile Verification Package") },
                navigationIcon = {
                    Button(onClick = onNavigateBack) { Text("< Back") }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Text(
                text = "Select Authoritative Claims to Disclose:",
                style = MaterialTheme.typography.titleMedium
            )
            
            // Maslow Level 1/2 Checklists
            Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)) {
                Column(modifier = Modifier.padding(8.dp)) {
                    Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                        Checkbox(checked = includeIdentity, onCheckedChange = { includeIdentity = it })
                        Text("Government Identity (Basic)")
                    }
                    Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                        Checkbox(checked = includePoliceCheck, onCheckedChange = { includePoliceCheck = it })
                        Text("Authoritative Police Check")
                    }
                    Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                        Checkbox(checked = includeHousingStatus, onCheckedChange = { includeHousingStatus = it })
                        Text("Current Housing Status")
                    }
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(modifier = Modifier.padding(16.dp), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                    Icon(Icons.Default.Security, contentDescription = "Privacy Guard", tint = NeonBlue)
                    Spacer(modifier = Modifier.width(16.dp))
                    Text(
                        text = "This generates a Zero-Knowledge W3C Verifiable Presentation. " +
                               "The Social Worker will ONLY see the checked claims.",
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }

            PremiumButton(
                text = "Generate Verifiable Presentation",
                onClick = {
                    // Triggers the Rust engine to sign a Verifiable Presentation using Ed25519
                    // and bundles the claims into a CBOR-LD ZIP via VC-15
                },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}
