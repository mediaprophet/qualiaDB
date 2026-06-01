package com.example.qualia.memes

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class MemeViewModel : ViewModel() {

    private lateinit var library: MemeLibrary

    private val _searchQuery = MutableStateFlow("")
    val searchQuery: StateFlow<String> = _searchQuery

    val memes: StateFlow<List<MemeEntry>> get() = library.memes

    val searchResults: StateFlow<List<Pair<MemeEntry, Int>>> =
        _searchQuery
            .debounce(300)
            .map { q -> if (::library.isInitialized) library.search(q) else emptyList() }
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    private val _selectedMeme = MutableStateFlow<MemeEntry?>(null)
    val selectedMeme: StateFlow<MemeEntry?> = _selectedMeme

    fun init(context: Context) {
        if (!::library.isInitialized) {
            library = MemeLibrary(context.applicationContext)
            viewModelScope.launch { library.load() }
        }
    }

    fun setSearchQuery(q: String) { _searchQuery.value = q }

    fun importImage(context: Context, uri: Uri) {
        viewModelScope.launch {
            library.importImage(uri)
        }
    }

    fun selectMeme(meme: MemeEntry?) { _selectedMeme.value = meme }

    fun delete(id: String) {
        viewModelScope.launch { library.delete(id) }
    }

    /** Expose library for MemePickerSheet and MemeIndexer integration. */
    fun getLibrary(): MemeLibrary? = if (::library.isInitialized) library else null
}
