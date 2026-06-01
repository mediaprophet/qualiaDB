package com.example.qualia

import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.lifecycle.lifecycleScope
import com.example.qualia.chat.ChatViewModel
import com.example.qualia.llm.LlmViewModel
import com.example.qualia.memes.MemeViewModel
import com.example.qualia.ontology.OntologyViewModel
import com.example.qualia.pdf.PdfViewModel
import com.example.qualia.theme.QualiaTheme
import com.example.qualia.update.UpdateChecker
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private val chatViewModel:     ChatViewModel     by viewModels()
    private val memeViewModel:     MemeViewModel     by viewModels()
    private val pdfViewModel:      PdfViewModel      by viewModels()
    private val ontologyViewModel: OntologyViewModel by viewModels()
    private val llmViewModel:      LlmViewModel      by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Initialise meme library with application context
        memeViewModel.init(this)

        // Wire meme search into chat for proactive meme suggestions
        chatViewModel.memeSearchFn = { query ->
            memeViewModel.getLibrary()?.search(query)?.firstOrNull()?.first
        }

        // Check for updates silently on launch
        lifecycleScope.launch {
            val update = UpdateChecker.checkForUpdate(BuildConfig.VERSION_NAME)
            if (update != null) {
                Toast.makeText(
                    this@MainActivity,
                    "Update available: v${update.version} — tap to download",
                    Toast.LENGTH_LONG,
                ).show()
            }
        }

        setContent {
            QualiaTheme {
                MainNavigation(
                    chatViewModel     = chatViewModel,
                    memeViewModel     = memeViewModel,
                    pdfViewModel      = pdfViewModel,
                    ontologyViewModel = ontologyViewModel,
                    llmViewModel      = llmViewModel,
                )
            }
        }
    }
}
