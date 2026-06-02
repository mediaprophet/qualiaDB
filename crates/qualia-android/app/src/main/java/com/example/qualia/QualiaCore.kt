package com.example.qualia

object QualiaCore {
    init {
        System.loadLibrary("qualia_core_db")
    }

    /**
     * Query the local ledger for transactions.
     * Returns a JSON string array of transactions.
     */
    external fun queryLedgerTransactions(): String

    /**
     * Insert a transaction into the local ledger.
     * Takes a JSON string representation of a transaction and returns a JSON response.
     */
    external fun insertLedgerTransaction(transactionJson: String): String

    /**
     * Insert a Quin transaction in CBOR format.
     */
    external fun insertCborQuin(cborBytes: ByteArray): Boolean
    
    /**
     * Commits the current project obligations, returning true if successful.
     */
    external fun commitProjectState(commitPayload: String): Boolean
    
    /**
     * Generates a git fast-export stream for the specified project.
     */
    external fun generateGitExport(projectId: String): String
    
    /**
     * Evaluates tax liabilities using the Sentinel VM based on the active Identity Nym (Verifiable Presentation).
     */
    external fun evaluateTaxLiability(identityNym: String): String
    
    /**
     * Inserts a spatial GPS log into the DB for tracking sessions.
     */
    external fun insertSpatialLog(spatialJson: String): Boolean
    
    /**
     * Calculates the business vs personal apportionment ratio using the GPU Spatial Sieve.
     */
    external fun calculateAssetApportionment(assetId: String): Double
}
