package com.example.qualia.health

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.qualia.ui.components.PremiumButton

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MedicationTrackerScreen(onNavigateBack: () -> Unit) {
    var medicationName by remember { mutableStateOf("") }
    var dosage by remember { mutableStateOf("") }
    var frequency by remember { mutableStateOf("") }
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Medication Tracker") },
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
            Text("Log Medication", style = MaterialTheme.typography.titleMedium)
            
            OutlinedTextField(
                value = medicationName,
                onValueChange = { medicationName = it },
                label = { Text("Medication Name (RxNorm)") },
                modifier = Modifier.fillMaxWidth()
            )
            
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = dosage,
                    onValueChange = { dosage = it },
                    label = { Text("Dosage (e.g. 50mg)") },
                    modifier = Modifier.weight(1f)
                )
                
                OutlinedTextField(
                    value = frequency,
                    onValueChange = { frequency = it },
                    label = { Text("Frequency (e.g. BID)") },
                    modifier = Modifier.weight(1f)
                )
            }
            
            Spacer(modifier = Modifier.weight(1f))
            
            PremiumButton(
                text = "Log to DB",
                onClick = {
                    // CborLdEncoder maps this to FHIR CBOR-LD and inserts into SLG Arena
                },
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}
