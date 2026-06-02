package com.example.qualia.identity

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.VerifiedUser
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.json.JSONObject

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CredentialWalletScreen(
    onNavigateBack: () -> Unit
) {
    // In a real app, this would be loaded from a secure encrypted enclave
    val mockCredentials = remember { 
        mutableStateListOf(
            "W3C Verifiable Credential: AU Tax Office (ABN 123456789)",
            "W3C Verifiable Credential: NSW Health (Clear STD Panel)",
            "Derived Claim: Over 18 (from Birth Registry)"
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identity Wallet (VCs)") },
                navigationIcon = {
                    Button(onClick = onNavigateBack) {
                        Text("< Back")
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(onClick = { /* Launch QR Scanner for new VC */ }) {
                Icon(Icons.Default.Add, contentDescription = "Add Credential")
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
                "Composable Identity Nyms", 
                fontSize = 20.sp, 
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(bottom = 16.dp)
            )
            
            Text(
                "Mix and match your verified credentials to generate tailored Presentations for specific projects or tax calculations.",
                modifier = Modifier.padding(bottom = 24.dp)
            )

            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(mockCredentials.size) { index ->
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Row(
                            modifier = Modifier.padding(16.dp),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Icon(Icons.Default.VerifiedUser, contentDescription = "Verified")
                            Spacer(modifier = Modifier.width(16.dp))
                            Text(mockCredentials[index], modifier = Modifier.weight(1f))
                        }
                    }
                }
                
                item {
                    Spacer(modifier = Modifier.height(24.dp))
                    Button(
                        onClick = { 
                            // Call CredentialManager.createVerifiablePresentation(...)
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Compose Active Presentation (VP)")
                    }
                }
            }
        }
    }
}
