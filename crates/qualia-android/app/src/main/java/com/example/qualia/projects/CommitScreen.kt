package com.example.qualia.projects

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CommitScreen(
    onCommitSuccess: () -> Unit,
    onNavigateBack: () -> Unit
) {
    var hours by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Log Obligation (Commit)") },
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
                .padding(16.dp)
        ) {
            OutlinedTextField(
                value = hours,
                onValueChange = { hours = it },
                label = { Text("Hours Contributed") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth()
            )
            
            Spacer(modifier = Modifier.height(16.dp))

            OutlinedTextField(
                value = description,
                onValueChange = { description = it },
                label = { Text("Commit Message / Description") },
                modifier = Modifier.fillMaxWidth(),
                minLines = 3
            )

            Spacer(modifier = Modifier.height(24.dp))

            Button(
                onClick = { 
                    // In production, this generates an Author-Scoped Merkle Signature
                    // via jni_bridge `commitProjectState`
                    onCommitSuccess() 
                },
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Sign & Commit to Ledger")
            }
        }
    }
}
