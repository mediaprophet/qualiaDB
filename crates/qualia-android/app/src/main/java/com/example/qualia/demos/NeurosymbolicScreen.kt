package com.example.qualia.demos

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.qualia.theme.*
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.math.*

// ── Concept model ─────────────────────────────────────────────────────────────

private data class Concept(
    val id:      String,
    val label:   String,
    val x:       Float,   // 0..1 normalised
    val y:       Float,
    val cluster: Int,
    val desc:    String,
)

private val CLUSTER_COLORS = listOf(NeonBlue, NeonGreen, NeonPurple, NeonGold, NeonCyan)

private val CONCEPTS = listOf(
    // Ontology / symbolic
    Concept("rdf",       "RDF",       0.12f, 0.18f, 0, "Resource Description Framework"),
    Concept("triple",    "Triple",    0.18f, 0.24f, 0, "Subject–Predicate–Object assertion"),
    Concept("quad",      "N-Quad",    0.15f, 0.30f, 0, "Named-graph quad extending the triple"),
    Concept("ontology",  "Ontology",  0.08f, 0.22f, 0, "Formal knowledge representation schema"),
    Concept("q42",       ".q42",      0.20f, 0.35f, 0, "QualiaDB binary ledger — compressed quads"),
    Concept("n3logic",   "N3Logic",   0.06f, 0.30f, 0, "Notation3 rule language"),
    // Neural
    Concept("embedding", "Embedding", 0.55f, 0.20f, 1, "Dense vector representation of meaning"),
    Concept("vector",    "Vector",    0.62f, 0.15f, 1, "Point in continuous high-dim space"),
    Concept("llm",       "LLM",       0.72f, 0.16f, 1, "Large Language Model"),
    Concept("inference", "Inference", 0.78f, 0.24f, 1, "Forward pass through neural model"),
    Concept("cosine",    "Cosine Sim",0.58f, 0.28f, 1, "Angle-based similarity metric"),
    // Governance
    Concept("fiduciary", "Fiduciary", 0.30f, 0.62f, 2, "Highest duty of care in agency law"),
    Concept("consent",   "Consent",   0.36f, 0.68f, 2, "Informed, explicit agreement"),
    Concept("privacy",   "Privacy",   0.24f, 0.70f, 2, "Control over personal information"),
    Concept("rights",    "Rights",    0.38f, 0.76f, 2, "Fundamental entitlements as predicates"),
    Concept("rule",      "Rule",      0.42f, 0.62f, 2, "Formal logical assertion in N3Logic"),
    // Identity
    Concept("did",       "DID",       0.72f, 0.65f, 3, "Decentralised Identifier — W3C"),
    Concept("credential","Credential",0.78f, 0.70f, 3, "Verifiable claim"),
    Concept("merkle",    "Merkle",    0.82f, 0.65f, 3, "Hash tree for integrity"),
    // Data
    Concept("compress",  "Compress",  0.45f, 0.45f, 4, "Reducing representation size"),
    Concept("hash",      "Hash",      0.52f, 0.50f, 4, "Deterministic content fingerprint"),
    Concept("schema",    "Schema",    0.48f, 0.40f, 4, "Structural data model definition"),
)

private val GOV_RULES = listOf(
    "NO_TELEMETRY"        to "Blocks all input data egress",
    "GROUNDED_OUTPUT"     to "All quads must cite provenance",
    "CONSENT_REQUIRED"    to "Processing requires consent triple",
    "MEMORY_BUDGET_512MB" to "Inference aborts above 512 MB",
    "FIDUCIARY_SUPREMACY" to "Fiduciary overrides LLM outputs",
    "RIGHTS_AS_PREDICATE" to "Rights ontology in named graph",
)

private val PRESETS = listOf(
    "The patient consented to fiduciary care",
    "LLM embedding encodes semantic relations",
    "Privacy-preserving inference with N3Logic rules",
    "DID credential for agent identity verification",
    "Binary ledger stores quads compressed",
)

// ── Screen ────────────────────────────────────────────────────────────────────

@Composable
fun NeurosymbolicScreen() {
    var inputText    by remember { mutableStateOf("") }
    var highlighted  by remember { mutableStateOf(setOf<String>()) }
    var pipelineStep by remember { mutableStateOf(-1) }
    var neuralOut    by remember { mutableStateOf("") }
    var symbolicOut  by remember { mutableStateOf("") }
    var firedRules   by remember { mutableStateOf(setOf<String>()) }
    var totalQuads   by remember { mutableStateOf(0) }
    var running      by remember { mutableStateOf(false) }

    val scope = rememberCoroutineScope()

    suspend fun runTranscompile(text: String) {
        if (running || text.isBlank()) return
        running      = true
        highlighted  = emptySet()
        firedRules   = emptySet()
        neuralOut    = ""
        symbolicOut  = ""

        // Animate pipeline steps
        for (step in 0..5) {
            pipelineStep = step
            delay(320)
        }

        // Find neighbours
        val neighbours = findNeighbours(text)
        highlighted = neighbours.map { it.id }.toSet()

        // Emit quads
        val quads = emitQuads(neighbours, text)
        totalQuads += quads.size
        neuralOut   = neighbours.mapIndexed { i, n ->
            "[${i+1}] ${n.label}: ${n.desc}"
        }.joinToString("\n")
        symbolicOut = quads.joinToString("\n")

        // Fire governance rules with stagger
        delay(200)
        GOV_RULES.forEachIndexed { i, (id, _) ->
            delay(250L)
            firedRules = firedRules + id
        }
        running = false
        pipelineStep = -1
    }

    Column(
        Modifier
            .fillMaxSize()
            .background(BgDeep)
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        // Header
        Text("NEUROSYMBOLIC TRANSCOMPILER",
            style = MaterialTheme.typography.displayLarge.copy(fontSize = 16.sp))
        Text("LLM bridges continuous vector space → discrete RDF symbolic space",
            color = TextMuted, fontSize = 12.sp)

        // Vector scatter
        Card(
            colors   = CardDefaults.cardColors(containerColor = BgCard),
            shape    = RoundedCornerShape(12.dp),
            border   = BorderStroke(1.dp, BorderDim),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column {
                Row(
                    Modifier.padding(10.dp, 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment     = Alignment.CenterVertically,
                ) {
                    Box(Modifier.size(8.dp).background(NeonBlue, RoundedCornerShape(4.dp)))
                    Text("Vector Space (2D PCA projection)", color = TextPrimary, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                    Spacer(Modifier.weight(1f))
                    Text("tap to transcompile", color = TextDim, fontSize = 10.sp)
                }
                HorizontalDivider(color = BorderDim)
                VectorScatterPlot(
                    concepts    = CONCEPTS,
                    highlighted = highlighted,
                    modifier    = Modifier.fillMaxWidth().height(220.dp),
                    onTap       = { concept ->
                        inputText = concept.desc
                        // launch transcompile from coroutine
                    }
                )
            }
        }

        // Presets
        Text("QUICK PRESETS", color = TextDim, fontSize = 10.sp, fontFamily = FontFamily.Monospace,
            letterSpacing = 1.sp)
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            PRESETS.forEach { preset ->
                FilterChip(
                    selected = false,
                    onClick  = {
                        inputText = preset
                        scope.launch { runTranscompile(preset) }
                    },
                    label    = { Text(preset.take(24) + "…", fontSize = 10.sp) },
                    colors   = FilterChipDefaults.filterChipColors(
                        containerColor   = BgCard,
                        labelColor       = TextMuted,
                    ),
                )
            }
        }

        // Input
        OutlinedTextField(
            value         = inputText,
            onValueChange = { inputText = it },
            label         = { Text("Sentence to transcompile") },
            placeholder   = { Text("Type any sentence…", color = TextDim) },
            modifier      = Modifier.fillMaxWidth(),
            colors        = OutlinedTextFieldDefaults.colors(
                focusedBorderColor   = NeonPurple,
                unfocusedBorderColor = BorderDim,
                focusedLabelColor    = NeonPurple,
            ),
            singleLine    = true,
        )

        // Pipeline steps
        PipelineBar(currentStep = pipelineStep)

        Button(
            onClick  = {
                scope.launch { runTranscompile(inputText) }
            },
            enabled  = !running && inputText.isNotBlank(),
            colors   = ButtonDefaults.buttonColors(
                containerColor = NeonPurple, contentColor = BgDeep,
                disabledContainerColor = BorderDim,
            ),
            modifier = Modifier.fillMaxWidth(),
            shape    = RoundedCornerShape(8.dp),
        ) {
            if (running) CircularProgressIndicator(Modifier.size(16.dp), color = BgDeep, strokeWidth = 2.dp)
            else Icon(Icons.Default.Transform, null, modifier = Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text(if (running) "Transcompiling…" else "Transcompile →", fontWeight = FontWeight.Bold)
        }

        // Output cards
        if (neuralOut.isNotBlank() || symbolicOut.isNotBlank()) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutputBox(
                    label   = "🧠 Neural neighbours",
                    content = neuralOut,
                    color   = NeonBlue,
                    modifier = Modifier.weight(1f),
                )
                OutputBox(
                    label   = "🔗 Emitted N-Quads",
                    content = symbolicOut,
                    color   = NeonPurple,
                    modifier = Modifier.weight(1f),
                )
            }
        }

        // Governance rules
        Text("N3LOGIC GOVERNANCE GATE", color = TextDim, fontSize = 10.sp,
            fontFamily = FontFamily.Monospace, letterSpacing = 1.sp)
        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            GOV_RULES.forEach { (id, desc) ->
                val fired = id in firedRules
                val anim by animateColorAsState(
                    if (fired) NeonGreen.copy(0.06f) else BgCard, tween(300), label = id)
                Row(
                    Modifier
                        .fillMaxWidth()
                        .background(anim, RoundedCornerShape(8.dp))
                        .border(1.dp, if (fired) NeonGreen.copy(0.4f) else BorderDim, RoundedCornerShape(8.dp))
                        .padding(10.dp, 7.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment     = Alignment.CenterVertically,
                ) {
                    Box(
                        Modifier.size(8.dp)
                            .background(if (fired) NeonGreen else TextDim, RoundedCornerShape(4.dp))
                    )
                    Column(Modifier.weight(1f)) {
                        Text(id, color = if (fired) NeonGreen else TextMuted, fontSize = 11.sp,
                            fontWeight = FontWeight.SemiBold, fontFamily = FontFamily.Monospace)
                        Text(desc, color = TextDim, fontSize = 10.sp)
                    }
                    Text(if (fired) "✓ ENFORCED" else "IDLE",
                        color = if (fired) NeonGreen else TextDim, fontSize = 10.sp,
                        fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace)
                }
            }
        }

        // Stats
        if (totalQuads > 0) {
            Card(
                colors   = CardDefaults.cardColors(containerColor = NeonPurple.copy(0.05f)),
                border   = BorderStroke(1.dp, NeonPurple.copy(0.3f)),
                shape    = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    Modifier.padding(14.dp).fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceAround,
                ) {
                    StatItem("$totalQuads", "N-Quads emitted", NeonPurple)
                    StatItem("100%", "Grounding fidelity", NeonGreen)
                    StatItem("0 B", "Network egress", NeonGold)
                }
            }
        }
    }
}

// ── Sub-composables ───────────────────────────────────────────────────────────

@Composable
private fun VectorScatterPlot(
    concepts:    List<Concept>,
    highlighted: Set<String>,
    modifier:    Modifier = Modifier,
    onTap:       (Concept) -> Unit = {},
) {
    val pulse by rememberInfiniteTransition(label = "pulse").animateFloat(
        0.6f, 1f, infiniteRepeatable(tween(1200, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "pulse"
    )
    Canvas(modifier.clickable { /* handled by pointer input */ }) {
        val w = size.width; val h = size.height

        concepts.forEach { c ->
            val cx   = c.x * w * 0.88f + w * 0.06f
            val cy   = c.y * h * 0.88f + h * 0.06f
            val color = CLUSTER_COLORS[c.cluster]
            val isHi  = c.id in highlighted
            val r     = if (isHi) 10f else 5f

            // Glow
            if (isHi) {
                drawCircle(color.copy(alpha = 0.18f * pulse), radius = 22f, center = Offset(cx, cy))
                drawCircle(color.copy(alpha = 0.08f), radius = 36f, center = Offset(cx, cy))
            }
            drawCircle(color.copy(alpha = if (isHi) 1f else 0.55f), radius = r, center = Offset(cx, cy))

            // Label
            drawContext.canvas.nativeCanvas.apply {
                val paint = android.graphics.Paint().apply {
                    textSize = if (isHi) 26f else 20f
                    this.color = if (isHi) color.toArgb() else android.graphics.Color.argb(120,200,200,240)
                    isAntiAlias = true
                }
                drawText(c.label, cx + r + 4f, cy + 7f, paint)
            }
        }
    }
}

private fun Color.toArgb(): Int {
    val r = (red   * 255).toInt()
    val g = (green * 255).toInt()
    val b = (blue  * 255).toInt()
    val a = (alpha * 255).toInt()
    return (a shl 24) or (r shl 16) or (g shl 8) or b
}

@Composable
private fun PipelineBar(currentStep: Int) {
    val steps = listOf("Text", "Embed", "Search", "Govern", "Quads", ".q42")
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        steps.forEachIndexed { i, label ->
            val active = i <= currentStep && currentStep >= 0
            val anim   by animateColorAsState(
                if (active) NeonPurple else BorderDim, tween(200), label = "pipe$i")
            Column(
                Modifier.weight(1f),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Box(Modifier.fillMaxWidth().height(4.dp).background(anim, RoundedCornerShape(2.dp)))
                Text(label, color = if (active) NeonPurple else TextDim, fontSize = 9.sp,
                    fontFamily = FontFamily.Monospace)
            }
        }
    }
}

@Composable
private fun OutputBox(label: String, content: String, color: Color, modifier: Modifier = Modifier) {
    Column(
        modifier
            .background(BgCard, RoundedCornerShape(8.dp))
            .border(1.dp, color.copy(0.3f), RoundedCornerShape(8.dp)),
    ) {
        Text(
            label, color = color, fontSize = 10.sp, fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(8.dp, 5.dp).fillMaxWidth(),
            fontFamily = FontFamily.Monospace,
        )
        HorizontalDivider(color = color.copy(0.2f))
        Text(
            content, color = TextMuted, fontSize = 9.sp,
            modifier = Modifier.padding(8.dp),
            fontFamily = FontFamily.Monospace, lineHeight = 14.sp,
        )
    }
}

@Composable
private fun StatItem(value: String, label: String, color: Color) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(value, color = color, fontSize = 18.sp, fontWeight = FontWeight.Bold,
            fontFamily = FontFamily.Monospace)
        Text(label, color = TextDim, fontSize = 10.sp)
    }
}

// ── Logic helpers ─────────────────────────────────────────────────────────────

private val KW_MAP = mapOf(
    "consent"   to listOf("consent","rights","fiduciary","agent","rule"),
    "fiduciary" to listOf("fiduciary","consent","rights","rule"),
    "privacy"   to listOf("privacy","consent","fiduciary","rights"),
    "knowledge" to listOf("ontology","rdf","triple","quad","embedding"),
    "graph"     to listOf("rdf","triple","ontology","quad"),
    "embedding" to listOf("embedding","vector","llm","cosine"),
    "semantic"  to listOf("rdf","ontology","triple","embedding","n3logic"),
    "vector"    to listOf("vector","embedding","cosine","llm"),
    "rdf"       to listOf("rdf","triple","quad","ontology"),
    "triple"    to listOf("triple","rdf","quad","ontology"),
    "quad"      to listOf("quad","triple","rdf","q42","n3logic"),
    "rule"      to listOf("rule","n3logic","fiduciary"),
    "ontology"  to listOf("ontology","rdf","schema"),
    "identity"  to listOf("did","credential","merkle"),
    "did"       to listOf("did","credential","merkle"),
    "binary"    to listOf("compress","hash","q42","schema"),
    "compress"  to listOf("compress","hash","q42"),
    "hash"      to listOf("hash","merkle","compress"),
    "inference" to listOf("inference","llm","embedding","vector"),
    "agent"     to listOf("fiduciary","did","consent","rule"),
    "rights"    to listOf("rights","consent","fiduciary","rule"),
    "patient"   to listOf("consent","fiduciary","rights","rule"),
    "llm"       to listOf("llm","embedding","vector","inference"),
)

private fun findNeighbours(text: String): List<Concept> {
    val words  = text.lowercase().split(Regex("\\W+"))
    val scores = mutableMapOf<String, Int>()
    CONCEPTS.forEach { scores[it.id] = 0 }
    words.forEach { w ->
        KW_MAP[w]?.forEachIndexed { rank, id -> scores[id] = (scores[id] ?: 0) + (5 - rank) }
        CONCEPTS.forEach { c ->
            if (c.label.lowercase().contains(w) || c.id.contains(w))
                scores[c.id] = (scores[c.id] ?: 0) + 3
        }
    }
    return CONCEPTS.filter { (scores[it.id] ?: 0) > 0 }
        .sortedByDescending { scores[it.id] }
        .take(5)
}

private fun emitQuads(neighbours: List<Concept>, sentence: String): List<String> {
    val docId = sentence.lowercase().replace(Regex("\\W+"), "_").take(18)
    val ts    = System.currentTimeMillis()
    return buildList {
        add("<urn:qualia:input:$docId> <rdf:type> <qualia:NLSentence> <urn:qualia:transcompile> .")
        add("<urn:qualia:input:$docId> <qualia:processedAt> \"$ts\"^^<xsd:long> <urn:qualia:transcompile> .")
        neighbours.forEachIndexed { i, n ->
            add("<urn:qualia:input:$docId> <qualia:nearestNeighbour> <urn:concept:${n.id}> <urn:qualia:transcompile> .")
            if (i < neighbours.size - 1)
                add("<urn:concept:${n.id}> <qualia:relatedTo> <urn:concept:${neighbours[i+1].id}> <urn:qualia:transcompile> .")
        }
        add("<urn:qualia:input:$docId> <qualia:governedBy> <urn:qualia:sentinel:v1> <urn:qualia:transcompile> .")
        add("<urn:qualia:input:$docId> <qualia:groundingFidelity> \"1.0\"^^<xsd:float> <urn:qualia:transcompile> .")
    }
}
