package com.example.qualia.pfm

import android.content.Context
import android.net.Uri
import java.io.BufferedReader
import java.io.InputStreamReader
import java.util.UUID

object StatementImporter {

    /**
     * Parses a generic bank statement CSV file.
     * Expects headers: Date, Payee, Amount
     */
    fun parseCsv(context: Context, uri: Uri): List<LedgerTransaction> {
        val transactions = mutableListOf<LedgerTransaction>()
        try {
            val inputStream = context.contentResolver.openInputStream(uri)
            val reader = BufferedReader(InputStreamReader(inputStream))
            
            // Skip header if it exists
            val firstLine = reader.readLine()
            val hasHeader = firstLine?.contains("amount", ignoreCase = true) == true
            
            if (!hasHeader && firstLine != null) {
                parseLine(firstLine)?.let { transactions.add(it) }
            }

            var line: String? = reader.readLine()
            while (line != null) {
                parseLine(line)?.let { transactions.add(it) }
                line = reader.readLine()
            }
            
            reader.close()
        } catch (e: Exception) {
            e.printStackTrace()
        }
        return transactions
    }

    private fun parseLine(line: String): LedgerTransaction? {
        // Very basic CSV split logic for prototyping.
        // Assumes format: "YYYY-MM-DD", "Payee Name", "-15.99"
        val parts = line.split(",").map { it.replace("\"", "").trim() }
        if (parts.size >= 3) {
            val date = parts[0]
            val payee = parts[1]
            val amountStr = parts[2]
            val amount = amountStr.toDoubleOrNull()
            
            if (amount != null) {
                return LedgerTransaction(
                    id = UUID.randomUUID().toString(),
                    date = date,
                    payee = payee,
                    amount = amount,
                    category = TransactionCategory.fromString(payee) // Try auto-categorize based on payee name
                )
            }
        }
        return null
    }
}
