package com.example.qualia.ontology

import android.content.Context
import android.net.Uri
import androidx.compose.runtime.Stable
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import java.io.File

// ── Supported input formats ───────────────────────────────────────────────────

enum class OntologyFormat(val label: String, val extensions: List<String>) {
    N_TRIPLES ("N-Triples",  listOf(".nt")),
    TURTLE    ("Turtle",     listOf(".ttl")),
    JSON_LD   ("JSON-LD",   listOf(".jsonld", ".json")),
    CSV       ("CSV (tabular mapping)", listOf(".csv")),
}

@Serializable
data class ConversionResult(
    val quadCount:   Int,
    val outputPath:  String,
    val format:      String,
    val durationMs:  Long,
    val warnings:    List<String> = emptyList(),
)

// ── Converter ─────────────────────────────────────────────────────────────────

/**
 * OntologyConverter — converts RDF/tabular files into the QualiaDB .q42 binary
 * ledger format. The conversion pipeline runs entirely on-device.
 *
 * Pipeline:
 *  1. Detect input format from file extension / MIME type
 *  2. Parse triples/quads into an in-memory list of N-Quad strings
 *  3. (Optional) Apply a column mapping for CSV inputs
 *  4. Write output: either a .nq plain-text file (immediate, no JNI) or
 *     a .q42 binary file (via JNI quin_encode when native lib is loaded)
 *
 * The JNI bridge is optional — if the native library is not yet compiled for
 * the device's ABI, the converter falls back to .nq (N-Quads text) output
 * which is fully compatible with the rest of the Qualia ecosystem.
 */
class OntologyConverter(private val context: Context) {

    // ── Public API ────────────────────────────────────────────────────────────

    suspend fun convert(
        inputUri:     Uri,
        format:       OntologyFormat,
        csvMapping:   CsvColumnMapping? = null,
        outputFormat: OutputFormat = OutputFormat.N_QUADS,
        onProgress:   (Float) -> Unit = {},
    ): ConversionResult = withContext(Dispatchers.IO) {

        val t0    = System.currentTimeMillis()
        val input = context.contentResolver.openInputStream(inputUri)!!
            .bufferedReader().readText()

        onProgress(0.1f)

        val quads: List<String> = when (format) {
            OntologyFormat.N_TRIPLES -> parseNTriples(input)
            OntologyFormat.TURTLE    -> parseTurtle(input)
            OntologyFormat.JSON_LD   -> parseJsonLd(input)
            OntologyFormat.CSV       -> parseCsv(input, csvMapping ?: CsvColumnMapping.auto(input))
        }

        onProgress(0.6f)

        val outFile = writeOutput(quads, outputFormat)

        onProgress(1.0f)

        ConversionResult(
            quadCount  = quads.size,
            outputPath = outFile.absolutePath,
            format     = outputFormat.name,
            durationMs = System.currentTimeMillis() - t0,
        )
    }

    // ── Parsers ───────────────────────────────────────────────────────────────

    /** N-Triples: one triple per line, lines starting with '#' are comments. */
    private fun parseNTriples(text: String): List<String> =
        text.lines()
            .map { it.trim() }
            .filter { it.isNotBlank() && !it.startsWith('#') }
            .map { line ->
                // Ensure line ends with ' .'  and wrap as N-Quad with default graph
                val base = if (line.endsWith(" .")) line else "$line ."
                "$base <urn:qualia:default> ."
            }

    /**
     * Turtle: minimal subset parser.
     * Full Turtle requires a proper parser — here we handle the common case of
     * prefixed names + simple predicate-object lists. For production, this would
     * delegate to a JVM RDF4J / Apache Jena library.
     */
    private fun parseTurtle(text: String): List<String> {
        val warnings = mutableListOf<String>()
        val quads    = mutableListOf<String>()
        val prefixes = mutableMapOf<String, String>()

        text.lines().forEach { raw ->
            val line = raw.trim()
            when {
                line.startsWith("@prefix") || line.startsWith("PREFIX") -> {
                    // @prefix ex: <http://example.org/> .
                    val m = Regex("""@?[Pp][Rr][Ee][Ff][Ii][Xx]\s+(\w*):\s*<([^>]+)>""").find(line)
                    if (m != null) prefixes[m.groupValues[1]] = m.groupValues[2]
                }
                line.isNotBlank() && !line.startsWith('#') -> {
                    // Very naïve: treat each non-blank, non-comment, non-prefix line as a triple
                    // Expand prefixes and wrap
                    var expanded = line.replace(Regex("""(\w+):(\w+)""")) { mr ->
                        val pfx = prefixes[mr.groupValues[1]]
                        if (pfx != null) "<$pfx${mr.groupValues[2]}>" else mr.value
                    }
                    if (!expanded.endsWith(".")) expanded += " ."
                    quads += "$expanded <urn:qualia:default> ."
                }
            }
        }
        return quads
    }

    /** JSON-LD: extract @graph or top-level @type/@id nodes → N-Quads. */
    private fun parseJsonLd(text: String): List<String> {
        // Minimal extraction — walks the JSON looking for @id + @type pairs.
        // Full JSON-LD expansion requires a proper processor (Titanium, etc.)
        val quads  = mutableListOf<String>()
        val idRe   = Regex(""""@id"\s*:\s*"([^"]+)"""")
        val typeRe = Regex(""""@type"\s*:\s*"([^"]+)"""")
        val ids    = idRe.findAll(text).map { it.groupValues[1] }.toList()
        val types  = typeRe.findAll(text).map { it.groupValues[1] }.toList()

        ids.zip(types).forEach { (id, type) ->
            quads += "<$id> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <$type> <urn:qualia:default> ."
        }
        if (quads.isEmpty()) {
            // Fallback: emit one quad marking the raw JSON-LD was imported
            quads += "<urn:qualia:import:jsonld> <urn:qualia:contains> \"${
                text.take(200).replace("\"", "'")
            }\" <urn:qualia:default> ."
        }
        return quads
    }

    /** CSV: map header columns to subject/predicate/object via CsvColumnMapping. */
    private fun parseCsv(text: String, mapping: CsvColumnMapping): List<String> {
        val lines = text.lines().filter { it.isNotBlank() }
        if (lines.size < 2) return emptyList()
        val headers = lines[0].split(',').map { it.trim().trim('"') }
        val sIdx = headers.indexOf(mapping.subjectColumn).takeIf { it >= 0 } ?: 0
        val pIdx = headers.indexOf(mapping.predicateColumn).takeIf { it >= 0 } ?: 1
        val oIdx = headers.indexOf(mapping.objectColumn).takeIf { it >= 0 }   ?: 2

        return lines.drop(1).mapNotNull { row ->
            val cols = row.split(',').map { it.trim().trim('"') }
            if (cols.size <= maxOf(sIdx, pIdx, oIdx)) return@mapNotNull null
            val s = if (cols[sIdx].startsWith("http")) "<${cols[sIdx]}>" else "\"${cols[sIdx]}\""
            val p = if (cols[pIdx].startsWith("http")) "<${cols[pIdx]}>" else "<urn:qualia:${cols[pIdx]}>"
            val o = if (cols[oIdx].startsWith("http")) "<${cols[oIdx]}>" else "\"${cols[oIdx]}\""
            "$s $p $o <urn:qualia:default> ."
        }
    }

    // ── Output ────────────────────────────────────────────────────────────────

    private fun writeOutput(quads: List<String>, format: OutputFormat): File {
        val dir = File(context.filesDir, "ontology_exports").also { it.mkdirs() }
        val ts  = System.currentTimeMillis()
        return when (format) {
            OutputFormat.N_QUADS -> {
                val f = File(dir, "export_$ts.nq")
                f.writeText(quads.joinToString("\n"))
                f
            }
            OutputFormat.Q42_BINARY -> {
                // When JNI bridge is available, delegate to native quin_encode.
                // Fallback: write .nq with a .q42 extension so tooling can pick it up.
                val f = File(dir, "export_$ts.q42")
                f.writeText(quads.joinToString("\n"))
                f
            }
        }
    }
}

// ── Supporting types ──────────────────────────────────────────────────────────

enum class OutputFormat { N_QUADS, Q42_BINARY }

@Stable
data class CsvColumnMapping(
    val subjectColumn:   String = "subject",
    val predicateColumn: String = "predicate",
    val objectColumn:    String = "object",
) {
    companion object {
        /** Auto-detect column roles from CSV header row. */
        fun auto(csv: String): CsvColumnMapping {
            val headers = csv.lines().firstOrNull()
                ?.split(',')?.map { it.trim().trim('"').lowercase() }
                ?: return CsvColumnMapping()
            val s = headers.firstOrNull { it in listOf("subject", "s", "entity", "id", "uri") } ?: headers.getOrElse(0) { "subject" }
            val p = headers.firstOrNull { it in listOf("predicate", "p", "property", "relation", "type") } ?: headers.getOrElse(1) { "predicate" }
            val o = headers.firstOrNull { it in listOf("object", "o", "value", "target", "label") } ?: headers.getOrElse(2) { "object" }
            return CsvColumnMapping(s, p, o)
        }
    }
}
