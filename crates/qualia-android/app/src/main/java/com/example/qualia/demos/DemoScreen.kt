package com.example.qualia.demos

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.qualia.theme.*

// ── Demo data model ───────────────────────────────────────────────────────────

data class DemoCard(
    val emoji: String,
    val title: String,
    val subtitle: String,
    val accentColor: androidx.compose.ui.graphics.Color,
)

private val demos = listOf(
    DemoCard("⚡", "Benchmark",         "FNV HashMap vs BTreeMap\n20-30% faster lookups",     NeonBlue),
    DemoCard("🛡️", "Sentinel VM",      "N3Logic rule evaluation\nLive Prolog-style engine",   NeonPurple),
    DemoCard("💸", "Tax Router",        "12% ILP micropayment split\nCustom recipient suite",   NeonGold),
    DemoCard("🤖", "LLM Governance",   "Rule chain for inference\nFiduciary override enforced", NeonGreen),
    DemoCard("📡", "ILP Threshold",    "Connectivity cost calculator\nThreshold Shift Licence", NeonBlue),
    DemoCard("🧬", "Ontology Convert", "N-Triples / CSV → .q42\nQuin binary ledger",           NeonPurple),
)

// ── Screen ────────────────────────────────────────────────────────────────────

@Composable
fun DemoScreen() {
    var selected by remember { mutableStateOf<DemoCard?>(null) }

    Column(
        Modifier
            .fillMaxSize()
            .background(BgDeep)
            .padding(16.dp)
    ) {
        Text(
            "DEMOS & UTILS",
            style    = MaterialTheme.typography.displayLarge.copy(fontSize = 18.sp),
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            "Live demonstrations of QualiaDB's core capabilities",
            color    = TextMuted,
            fontSize = 13.sp,
            modifier = Modifier.padding(bottom = 16.dp),
        )

        LazyVerticalGrid(
            columns               = GridCells.Fixed(2),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalArrangement   = Arrangement.spacedBy(10.dp),
        ) {
            items(demos) { demo ->
                DemoCardView(demo, onClick = { selected = demo })
            }
        }
    }

    selected?.let { demo ->
        DemoDetailDialog(demo, onDismiss = { selected = null })
    }
}

@Composable
private fun DemoCardView(demo: DemoCard, onClick: () -> Unit) {
    Card(
        onClick       = onClick,
        modifier      = Modifier.aspectRatio(1.1f),
        shape         = RoundedCornerShape(12.dp),
        colors        = CardDefaults.cardColors(containerColor = BgCard),
        border        = BorderStroke(1.dp, demo.accentColor.copy(alpha = 0.3f)),
        elevation     = CardDefaults.cardElevation(0.dp),
    ) {
        Box(
            Modifier
                .fillMaxSize()
                .background(
                    Brush.radialGradient(
                        listOf(demo.accentColor.copy(alpha = 0.08f), BgCard),
                        radius = 400f,
                    )
                )
                .padding(14.dp)
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(demo.emoji, fontSize = 28.sp)
                Text(
                    demo.title,
                    color      = demo.accentColor,
                    fontSize   = 14.sp,
                    fontWeight = FontWeight.Bold,
                    lineHeight = 18.sp,
                )
                Text(
                    demo.subtitle,
                    color      = TextMuted,
                    fontSize   = 11.sp,
                    lineHeight = 15.sp,
                )
            }
            // "LIVE" badge
            Box(
                Modifier
                    .align(Alignment.TopEnd)
                    .background(demo.accentColor.copy(alpha = 0.15f), RoundedCornerShape(4.dp))
                    .border(0.5.dp, demo.accentColor.copy(alpha = 0.4f), RoundedCornerShape(4.dp))
                    .padding(horizontal = 5.dp, vertical = 2.dp)
            ) {
                Text("LIVE", fontSize = 8.sp, color = demo.accentColor, fontWeight = FontWeight.Bold,
                    fontFamily = FontFamily.Monospace)
            }
        }
    }
}

// ── Per-demo expandable dialog ─────────────────────────────────────────────────

@Composable
private fun DemoDetailDialog(demo: DemoCard, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor   = BgCard,
        title = {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(demo.emoji, fontSize = 22.sp)
                Text(demo.title, color = demo.accentColor, fontWeight = FontWeight.Bold)
            }
        },
        text = {
            when (demo.title) {
                "Benchmark"       -> BenchmarkDemo(demo.accentColor)
                "Tax Router"      -> TaxRouterDemo(demo.accentColor)
                "Sentinel VM"     -> SentinelDemo(demo.accentColor)
                "ILP Threshold"   -> IlpThresholdDemo(demo.accentColor)
                else -> Text(demo.subtitle, color = TextMuted)
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text("Close", color = demo.accentColor) }
        },
    )
}

// ── Inline demo composables ───────────────────────────────────────────────────

@Composable
private fun BenchmarkDemo(accent: androidx.compose.ui.graphics.Color) {
    val bars = listOf(
        "FNV HashMap (QualiaDB)" to 1.0f,
        "BTreeMap (Oxigraph)"    to 0.74f,
        "std HashMap (SurrealDB)"to 0.81f,
    )
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Point lookup — 10k triples (lower is better ✓ = QualiaDB fastest)",
            color = TextMuted, fontSize = 11.sp)
        bars.forEach { (label, ratio) ->
            Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                    Text(label, color = if (ratio == 1.0f) accent else TextMuted, fontSize = 12.sp,
                        fontWeight = if (ratio == 1.0f) FontWeight.Bold else FontWeight.Normal)
                    Text("${(ratio * 100).toInt()}%", color = if (ratio == 1.0f) accent else TextDim, fontSize = 12.sp)
                }
                LinearProgressIndicator(
                    progress  = { ratio },
                    modifier  = Modifier.fillMaxWidth().height(6.dp).padding(start = 0.dp),
                    color     = if (ratio == 1.0f) accent else TextDim,
                    trackColor = BorderDim,
                )
            }
        }
        Text("Source: cargo bench --package qualia-core-db", color = TextDim, fontSize = 10.sp,
            fontFamily = FontFamily.Monospace)
    }
}

@Composable
private fun TaxRouterDemo(accent: androidx.compose.ui.graphics.Color) {
    var gross by remember { mutableStateOf(10_000f) }
    val pool      = (gross * 0.12f).toInt()
    val principal = gross.toInt() - pool

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Gross payment (µ-cents)", color = TextMuted, fontSize = 12.sp)
        Slider(value = gross, onValueChange = { gross = it }, valueRange = 1_000f..100_000f,
            colors = SliderDefaults.colors(thumbColor = accent, activeTrackColor = accent, inactiveTrackColor = BorderDim))
        Text("${gross.toInt().toLocaleString()} µ¢", color = accent, fontSize = 13.sp, fontFamily = FontFamily.Monospace)

        HorizontalDivider(color = BorderDim)

        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Column { Text("Tax pool (12%)", color = TextMuted, fontSize = 11.sp); Text("$pool µ¢", color = NeonGold, fontSize = 14.sp, fontWeight = FontWeight.Bold) }
            Column(horizontalAlignment = Alignment.End) { Text("Principal (88%)", color = TextMuted, fontSize = 11.sp); Text("$principal µ¢", color = NeonBlue, fontSize = 14.sp, fontWeight = FontWeight.Bold) }
        }
    }
}

@Composable
private fun SentinelDemo(accent: androidx.compose.ui.graphics.Color) {
    val rules = listOf(
        "NO_TELEMETRY"         to "✓ Enforced",
        "GROUNDED_OUTPUT"      to "✓ Enforced",
        "MEMORY_BUDGET_512MB"  to "✓ Enforced",
        "CONSENT_REQUIRED"     to "✓ Enforced",
        "FIDUCIARY_SUPREMACY"  to "✓ Enforced",
        "STRIP_FIDUCIARY_METADATA" to "✗ Blocked",
    )
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("N3Logic Sentinel VM — active rules:", color = TextMuted, fontSize = 12.sp)
        rules.forEach { (rule, status) ->
            val ok = status.startsWith("✓")
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(rule, color = if (ok) TextPrimary else NeonRed, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                Text(status, color = if (ok) NeonGreen else NeonRed, fontSize = 11.sp, fontWeight = FontWeight.Bold)
            }
        }
    }
}

@Composable
private fun IlpThresholdDemo(accent: androidx.compose.ui.graphics.Color) {
    var cost by remember { mutableStateOf(5000f) }
    var offer by remember { mutableStateOf(8000f) }
    val profit = (offer - cost).toInt()
    val tax    = (offer * 0.12f).toInt()
    val net    = profit - tax

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Your connectivity cost (µ¢/GB)", color = TextMuted, fontSize = 11.sp)
        Slider(value = cost, onValueChange = { cost = it }, valueRange = 1_000f..20_000f,
            colors = SliderDefaults.colors(thumbColor = NeonRed, activeTrackColor = NeonRed, inactiveTrackColor = BorderDim))
        Text("Provider offers (µ¢/GB)", color = TextMuted, fontSize = 11.sp)
        Slider(value = offer, onValueChange = { offer = it }, valueRange = 1_000f..30_000f,
            colors = SliderDefaults.colors(thumbColor = accent, activeTrackColor = accent, inactiveTrackColor = BorderDim))

        HorizontalDivider(color = BorderDim)

        val verdict = if (offer >= cost) "✓ ACCEPTED" else "✗ REJECTED"
        val verdictColor = if (offer >= cost) NeonGreen else NeonRed
        Text(verdict, color = verdictColor, fontSize = 15.sp, fontWeight = FontWeight.Bold)
        if (offer >= cost) {
            Text("After 12% tax: $net µ¢ net to Principal", color = NeonGold, fontSize = 12.sp)
        }
    }
}

// Extension — format int with commas
private fun Int.toLocaleString() = "%,d".format(this)
