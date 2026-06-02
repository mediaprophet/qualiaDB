package com.example.qualia.pfm

/**
 * Localized taxonomy for categorizing financial expenditures and income.
 * Maps to ontological nodes in the Qualia Quin graph.
 */
enum class TransactionCategory(val displayName: String, val isIncome: Boolean) {
    // Income
    CLIENT_PAYMENT("Client Payment", true),
    WAGE("Wage", true),
    INVESTMENT_RETURN("Investment Return", true),
    MISC_INCOME("Misc Income", true),

    // Expenses
    SOFTWARE("Software Subscriptions", false),
    HARDWARE("Hardware & Equipment", false),
    TRAVEL("Travel & Transport", false),
    MEALS("Meals & Entertainment", false),
    UTILITIES("Utilities", false),
    RENT("Rent & Lease", false),
    MISC_EXPENSE("Misc Expense", false),
    TAX("Tax Withheld", false);

    companion object {
        fun fromString(value: String): TransactionCategory {
            return values().find { it.name.equals(value, ignoreCase = true) || it.displayName.equals(value, ignoreCase = true) }
                ?: if (value.contains("income", ignoreCase = true)) MISC_INCOME else MISC_EXPENSE
        }
    }
}

data class LedgerTransaction(
    val id: String,
    val date: String,
    val payee: String,
    val amount: Double,
    val category: TransactionCategory,
    val currency: String = "USD"
)
