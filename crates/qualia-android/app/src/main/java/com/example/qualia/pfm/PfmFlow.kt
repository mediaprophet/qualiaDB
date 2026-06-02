package com.example.qualia.pfm

import androidx.compose.runtime.*

enum class PfmDest {
    LEDGER, RECONCILIATION, REPORTING
}

@Composable
fun PfmFlow() {
    var currentDest by remember { mutableStateOf(PfmDest.LEDGER) }

    when (currentDest) {
        PfmDest.LEDGER -> LedgerScreen(
            onNavigateToReconciliation = { currentDest = PfmDest.RECONCILIATION },
            onNavigateToReporting = { currentDest = PfmDest.REPORTING }
        )
        PfmDest.RECONCILIATION -> ReconciliationScreen(
            onNavigateBack = { currentDest = PfmDest.LEDGER }
        )
        PfmDest.REPORTING -> ReportingScreen(
            onNavigateBack = { currentDest = PfmDest.LEDGER }
        )
    }
}
