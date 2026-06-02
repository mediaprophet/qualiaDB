package com.example.qualia.pfm

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.patrykandpatrick.vico.compose.axis.horizontal.rememberBottomAxis
import com.patrykandpatrick.vico.compose.axis.vertical.rememberStartAxis
import com.patrykandpatrick.vico.compose.chart.Chart
import com.patrykandpatrick.vico.compose.chart.column.rememberColumnChart
import com.patrykandpatrick.vico.core.entry.ChartEntryModelProducer
import com.patrykandpatrick.vico.core.entry.FloatEntry

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReportingScreen(
    onNavigateBack: () -> Unit,
    viewModel: LedgerViewModel = viewModel()
) {
    val transactions by viewModel.transactions.collectAsState()

    // Process transactions into chart entries
    // For simplicity, we just aggregate income vs expenses
    var totalIncome = 0.0f
    var totalExpense = 0.0f
    
    transactions.forEach { tx ->
        if (tx.amount > 0) {
            totalIncome += tx.amount.toFloat()
        } else {
            totalExpense += kotlin.math.abs(tx.amount.toFloat())
        }
    }

    val chartEntryModelProducer = ChartEntryModelProducer(
        listOf(
            FloatEntry(x = 0f, y = totalIncome),
            FloatEntry(x = 1f, y = totalExpense)
        )
    )

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Financial Reports") },
                navigationIcon = {
                    Button(onClick = onNavigateBack) {
                        Text("< Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(16.dp)
                .verticalScroll(rememberScrollState())
        ) {
            Text(
                text = "Profit & Loss (Overall)",
                fontSize = 20.sp,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(bottom = 16.dp)
            )
            
            // Bar Chart rendering using Vico
            Chart(
                chart = rememberColumnChart(),
                chartModelProducer = chartEntryModelProducer,
                startAxis = rememberStartAxis(),
                bottomAxis = rememberBottomAxis(
                    valueFormatter = { value, _ ->
                        if (value == 0f) "Income" else if (value == 1f) "Expenses" else ""
                    }
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(250.dp)
            )

            Spacer(modifier = Modifier.height(32.dp))
            
            // Text Summary
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Total Income: ${formatCurrency(totalIncome.toDouble())}", color = androidx.compose.ui.graphics.Color(0xFF4CAF50))
                    Spacer(modifier = Modifier.height(8.dp))
                    Text("Total Expenses: ${formatCurrency(totalExpense.toDouble())}", color = androidx.compose.ui.graphics.Color(0xFFF44336))
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        "Net: ${formatCurrency((totalIncome - totalExpense).toDouble())}",
                        fontWeight = FontWeight.Bold,
                        fontSize = 18.sp
                    )
                }
            }
        }
    }
}
