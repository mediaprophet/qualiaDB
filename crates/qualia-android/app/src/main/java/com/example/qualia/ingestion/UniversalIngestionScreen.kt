package com.example.qualia.ingestion

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.UploadFile
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.example.qualia.ui.components.PremiumButton

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UniversalIngestionScreen(onNavigateBack: () -> Unit) {
    val context = LocalContext.current
    var selectedUri by remember { mutableStateOf<Uri?>(null) }
    var selectedContext by remember { mutableStateOf("PFM Tax Receipt") }
    
    val contextOptions = listOf("PFM Tax Receipt", "Meme Indexing", "Heraldry Document")

    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? -> selectedUri = uri }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Universal Ingestion") },
                navigationIcon = {
                    Button(onClick = onNavigateBack) {
                        Text("< Back")
                    }
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
            Text("Extract CBOR-LD structured data via Local VLM.", style = MaterialTheme.typography.titleMedium)

            OutlinedCard(modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
                Column(
                    modifier = Modifier.padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Icon(Icons.Default.UploadFile, contentDescription = null, modifier = Modifier.size(48.dp))
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(if (selectedUri != null) "File Selected: ${selectedUri?.lastPathSegment}" else "No File Selected")
                    Spacer(modifier = Modifier.height(16.dp))
                    PremiumButton(
                        text = "Select Image or PDF",
                        onClick = { filePickerLauncher.launch("*/*") }
                    )
                }
            }

            Text("Extraction Context:")
            contextOptions.forEach { option ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    RadioButton(
                        selected = selectedContext == option,
                        onClick = { selectedContext = option }
                    )
                    Text(option)
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            PremiumButton(
                text = "Run VLM Extraction",
                onClick = {
                    // This is where we will pipe the URI and selectedContext into PdfScanner / EdgeExtractor
                },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}
