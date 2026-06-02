package com.example.qualia.debug

import com.example.qualia.health.PhrOntologyManager
import com.example.qualia.health.PhrOntologyManager.AssessmentType

/**
 * Legacy Support: Demo Mode (vault-mock.js equivalent).
 * Populates the SLG Arena with synthetic personas and safe mock data for testing
 * the UI components without requiring actual clinical data or real user keys.
 */
class DemoModeManager(private val ontologyManager: PhrOntologyManager) {

    fun isDemoActive(): Boolean {
        // Can be tied to a BuildConfig flag or a specific Demo PIN (e.g., '0000')
        return true
    }

    fun injectSyntheticPersonas() {
        if (!isDemoActive()) return

        // Inject Synthetic Mental Health Assessments
        val mockAssessment = PhrOntologyManager.PsychologyAssessment(
            assessmentId = "demo_k10_1",
            type = AssessmentType.K_10,
            scores = mapOf("total" to 15),
            clinicalNotes = "Synthetic user reporting mild distress. Mock data.",
            spatioTemporalContext = "Minkowski Block A"
        )
        
        ontologyManager.encodeToCborLd(mockAssessment)
        
        // Inject Synthetic Verified Communications Contacts
        // e.g. "Alice (Social Worker)", "Bob (Housing Agent)"

        println("DEMO MODE: Synthetic personas and mock data injected successfully.")
    }
}
