package com.example.qualia.pfm

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.qualia.QualiaCore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

class LedgerViewModel : ViewModel() {
    private val _transactions = MutableStateFlow<List<LedgerTransaction>>(emptyList())
    val transactions: StateFlow<List<LedgerTransaction>> = _transactions.asStateFlow()

    private val _totalBalance = MutableStateFlow(0.0)
    val totalBalance: StateFlow<Double> = _totalBalance.asStateFlow()

    init {
        loadLedger()
    }

    fun loadLedger() {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                // Call Rust JNI to get transactions
                val jsonStr = QualiaCore.queryLedgerTransactions()
                val jsonArray = JSONArray(jsonStr)
                val txList = mutableListOf<LedgerTransaction>()
                var balance = 0.0

                for (i in 0 until jsonArray.length()) {
                    val obj = jsonArray.getJSONObject(i)
                    val tx = LedgerTransaction(
                        id = obj.getString("id"),
                        date = obj.getString("date"),
                        payee = obj.getString("payee"),
                        amount = obj.getDouble("amount"),
                        category = TransactionCategory.fromString(obj.getString("category")),
                        currency = obj.getString("currency")
                    )
                    txList.add(tx)
                    balance += tx.amount
                }

                _transactions.value = txList.sortedByDescending { it.date }
                _totalBalance.value = balance
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }
}
