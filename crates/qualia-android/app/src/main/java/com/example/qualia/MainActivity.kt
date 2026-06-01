package com.example.qualia

import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.lifecycle.lifecycleScope
import com.example.qualia.chat.ChatViewModel
import com.example.qualia.memes.MemeViewModel
import com.example.qualia.theme.QualiaTheme
import com.example.qualia.update.UpdateChecker
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private val chatViewModel:  ChatViewModel  by viewModels()
    private val memeViewModel:  MemeViewModel  by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Initialise meme library with context
        memeViewModel.init(this)

        // Wire meme search into chat
        chatViewModel.memeSearchFn = { query ->
            memeViewModel.getLibrary()?.search(query)?.firstOrNull()?.first
        }

        // Check for updates on launch (non-blocking)
        lifecycleScope.launch {
            val currentVersion = BuildConfig.VERSION_NAME
            val update = UpdateChecker.checkForUpdate(currentVersion)
            if (update != null) {
                Toast.makeText(
                    this@MainActivity,
                    "Update available: v${update.version} — tap to download",
                    Toast.LENGTH_LONG,
                ).also { toast ->
                    toast.show()
                    // Simple tap on the toast opens releases page
                    // (full Snackbar wired in the composable layer)
                }
            }
        }

        setContent {
            QualiaTheme {
                MainNavigation(
                    chatViewModel = chatViewModel,
                    memeViewModel = memeViewModel,
                )
            }
        }
    }
}
