package com.example.qualia.ontology

import android.content.Context
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Manages loading semantic ontologies (.q42 files) to enrich the LLM 
 * context window during Vision and OCR extraction.
 */
class OntologyManager(private val context: Context) {

    // JNI bindings for injecting the .q42 binary representation into the DB
    private external fun loadQ42Ontology(filePath: String): Boolean

    suspend fun importOntology(q42File: File): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                // Ensure the file is safely stored in our internal data dir
                val destFile = File(context.filesDir, "ontologies/${q42File.name}")
                destFile.parentFile?.mkdirs()
                
                if (q42File.absolutePath != destFile.absolutePath) {
                    q42File.copyTo(destFile, overwrite = true)
                }

                // Inject into the Qualia native Core DB
                loadQ42Ontology(destFile.absolutePath)
            } catch (e: Exception) {
                e.printStackTrace()
                false
            }
        }
    }
}
