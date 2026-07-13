//! Double-entry accounting kernel.
//!
//! Amounts are signed integer minor units (for example cents). Posting debits
//! and credits must be finite ledger amounts: non-negative, one-sided, and
//! balanced per journal entry.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Asset = 0,
    Liability = 1,
    Equity = 2,
    Revenue = 3,
    Expense = 4,
}

impl AccountType {
    pub const fn is_debit_normal(self) -> bool {
        matches!(self, AccountType::Asset | AccountType::Expense)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Account {
    pub id: u64,
    pub account_type: AccountType,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posting {
    pub entry_id: u64,
    pub account_id: u64,
    pub debit: i64,
    pub credit: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: u64,
    pub posting_start: usize,
    pub posting_len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountBalance {
    pub account_id: u64,
    pub account_type: AccountType,
    /// Positive values are favorable to the account's normal balance side.
    ///
    /// Assets and expenses compute `debits - credits`; liabilities, equity,
    /// and revenue compute `credits - debits`.
    pub balance: i128,
    pub debit_total: i128,
    pub credit_total: i128,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialBalance {
    pub total_debits: i128,
    pub total_credits: i128,
    pub debit_balance_total: i128,
    pub credit_balance_total: i128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountingError {
    InvalidPosting,
    EntryNotFound,
    UnbalancedEntry {
        entry_id: u64,
        total_debits: i128,
        total_credits: i128,
    },
    UnknownAccount {
        account_id: u64,
    },
    OutputBufferTooSmall,
}

fn validate_posting_amounts(posting: &Posting) -> Result<(), AccountingError> {
    if posting.debit < 0 || posting.credit < 0 {
        return Err(AccountingError::InvalidPosting);
    }
    if (posting.debit == 0 && posting.credit == 0) || (posting.debit > 0 && posting.credit > 0) {
        return Err(AccountingError::InvalidPosting);
    }
    Ok(())
}

fn find_account(accounts: &[Account], account_id: u64) -> Option<Account> {
    for account in accounts {
        if account.id == account_id {
            return Some(*account);
        }
    }
    None
}

fn validate_known_accounts(
    accounts: &[Account],
    postings: &[Posting],
) -> Result<(), AccountingError> {
    for posting in postings {
        validate_posting_amounts(posting)?;
        if find_account(accounts, posting.account_id).is_none() {
            return Err(AccountingError::UnknownAccount {
                account_id: posting.account_id,
            });
        }
    }
    Ok(())
}

/// Validate that all postings belonging to `entry_id` form a balanced entry.
pub fn validate_balanced_entry(entry_id: u64, postings: &[Posting]) -> Result<(), AccountingError> {
    let mut total_debits = 0i128;
    let mut total_credits = 0i128;
    let mut found = false;

    for posting in postings {
        if posting.entry_id != entry_id {
            continue;
        }
        validate_posting_amounts(posting)?;
        found = true;
        total_debits += posting.debit as i128;
        total_credits += posting.credit as i128;
    }

    if !found {
        return Err(AccountingError::EntryNotFound);
    }
    if total_debits != total_credits {
        return Err(AccountingError::UnbalancedEntry {
            entry_id,
            total_debits,
            total_credits,
        });
    }
    Ok(())
}

/// Validate a journal entry's declared posting slice and balance.
pub fn validate_journal_entry(
    entry: &JournalEntry,
    postings: &[Posting],
) -> Result<(), AccountingError> {
    let end = entry
        .posting_start
        .checked_add(entry.posting_len)
        .ok_or(AccountingError::InvalidPosting)?;
    if entry.posting_len == 0 || end > postings.len() {
        return Err(AccountingError::InvalidPosting);
    }

    let slice = &postings[entry.posting_start..end];
    for posting in slice {
        if posting.entry_id != entry.id {
            return Err(AccountingError::InvalidPosting);
        }
    }
    validate_balanced_entry(entry.id, slice)
}

/// Validate all declared journal entries.
pub fn validate_journal_entries(
    entries: &[JournalEntry],
    postings: &[Posting],
) -> Result<(), AccountingError> {
    for entry in entries {
        validate_journal_entry(entry, postings)?;
    }
    Ok(())
}

/// Compute balances for every supplied account into caller-owned output.
pub fn account_balances_into(
    accounts: &[Account],
    postings: &[Posting],
    out: &mut [AccountBalance],
) -> Result<usize, AccountingError> {
    if out.len() < accounts.len() {
        return Err(AccountingError::OutputBufferTooSmall);
    }
    validate_known_accounts(accounts, postings)?;

    for (idx, account) in accounts.iter().enumerate() {
        let mut debit_total = 0i128;
        let mut credit_total = 0i128;

        for posting in postings {
            if posting.account_id == account.id {
                debit_total += posting.debit as i128;
                credit_total += posting.credit as i128;
            }
        }

        let balance = if account.account_type.is_debit_normal() {
            debit_total - credit_total
        } else {
            credit_total - debit_total
        };

        out[idx] = AccountBalance {
            account_id: account.id,
            account_type: account.account_type,
            balance,
            debit_total,
            credit_total,
        };
    }

    Ok(accounts.len())
}

/// Aggregate posting totals and account-side trial-balance totals.
pub fn trial_balance(
    accounts: &[Account],
    postings: &[Posting],
) -> Result<TrialBalance, AccountingError> {
    validate_known_accounts(accounts, postings)?;

    let mut total_debits = 0i128;
    let mut total_credits = 0i128;
    for posting in postings {
        total_debits += posting.debit as i128;
        total_credits += posting.credit as i128;
    }
    if total_debits != total_credits {
        return Err(AccountingError::UnbalancedEntry {
            entry_id: 0,
            total_debits,
            total_credits,
        });
    }

    let mut debit_balance_total = 0i128;
    let mut credit_balance_total = 0i128;
    for account in accounts {
        let mut debit_total = 0i128;
        let mut credit_total = 0i128;

        for posting in postings {
            if posting.account_id == account.id {
                debit_total += posting.debit as i128;
                credit_total += posting.credit as i128;
            }
        }

        let raw_debit_balance = debit_total - credit_total;
        if raw_debit_balance >= 0 {
            debit_balance_total += raw_debit_balance;
        } else {
            credit_balance_total += -raw_debit_balance;
        }
    }

    Ok(TrialBalance {
        total_debits,
        total_credits,
        debit_balance_total,
        credit_balance_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASH: u64 = 1;
    const PAYABLE: u64 = 2;
    const EQUITY: u64 = 3;
    const REVENUE: u64 = 4;
    const EXPENSE: u64 = 5;

    fn accounts() -> [Account; 5] {
        [
            Account {
                id: CASH,
                account_type: AccountType::Asset,
            },
            Account {
                id: PAYABLE,
                account_type: AccountType::Liability,
            },
            Account {
                id: EQUITY,
                account_type: AccountType::Equity,
            },
            Account {
                id: REVENUE,
                account_type: AccountType::Revenue,
            },
            Account {
                id: EXPENSE,
                account_type: AccountType::Expense,
            },
        ]
    }

    #[test]
    fn balanced_entry_is_accepted() {
        let postings = [
            Posting {
                entry_id: 10,
                account_id: CASH,
                debit: 1_000,
                credit: 0,
            },
            Posting {
                entry_id: 10,
                account_id: REVENUE,
                debit: 0,
                credit: 1_000,
            },
        ];

        assert_eq!(validate_balanced_entry(10, &postings), Ok(()));
    }

    #[test]
    fn unbalanced_entry_is_rejected() {
        let postings = [
            Posting {
                entry_id: 11,
                account_id: CASH,
                debit: 1_000,
                credit: 0,
            },
            Posting {
                entry_id: 11,
                account_id: REVENUE,
                debit: 0,
                credit: 900,
            },
        ];

        assert_eq!(
            validate_balanced_entry(11, &postings),
            Err(AccountingError::UnbalancedEntry {
                entry_id: 11,
                total_debits: 1_000,
                total_credits: 900,
            })
        );
    }

    #[test]
    fn account_balance_signs_follow_normal_balance_type() {
        let ledger_accounts = accounts();
        let postings = [
            Posting {
                entry_id: 20,
                account_id: CASH,
                debit: 1_000,
                credit: 0,
            },
            Posting {
                entry_id: 20,
                account_id: REVENUE,
                debit: 0,
                credit: 1_000,
            },
            Posting {
                entry_id: 21,
                account_id: EXPENSE,
                debit: 250,
                credit: 0,
            },
            Posting {
                entry_id: 21,
                account_id: CASH,
                debit: 0,
                credit: 250,
            },
            Posting {
                entry_id: 22,
                account_id: PAYABLE,
                debit: 0,
                credit: 400,
            },
            Posting {
                entry_id: 22,
                account_id: EXPENSE,
                debit: 400,
                credit: 0,
            },
            Posting {
                entry_id: 23,
                account_id: CASH,
                debit: 600,
                credit: 0,
            },
            Posting {
                entry_id: 23,
                account_id: EQUITY,
                debit: 0,
                credit: 600,
            },
        ];
        let mut out = [AccountBalance {
            account_id: 0,
            account_type: AccountType::Asset,
            balance: 0,
            debit_total: 0,
            credit_total: 0,
        }; 5];

        assert_eq!(
            account_balances_into(&ledger_accounts, &postings, &mut out),
            Ok(5)
        );
        assert_eq!(out[0].balance, 1_350);
        assert_eq!(out[1].balance, 400);
        assert_eq!(out[2].balance, 600);
        assert_eq!(out[3].balance, 1_000);
        assert_eq!(out[4].balance, 650);
    }

    #[test]
    fn trial_balance_totals_match() {
        let ledger_accounts = accounts();
        let postings = [
            Posting {
                entry_id: 30,
                account_id: CASH,
                debit: 1_000,
                credit: 0,
            },
            Posting {
                entry_id: 30,
                account_id: REVENUE,
                debit: 0,
                credit: 1_000,
            },
            Posting {
                entry_id: 31,
                account_id: EXPENSE,
                debit: 300,
                credit: 0,
            },
            Posting {
                entry_id: 31,
                account_id: CASH,
                debit: 0,
                credit: 300,
            },
        ];

        assert_eq!(
            trial_balance(&ledger_accounts, &postings),
            Ok(TrialBalance {
                total_debits: 1_300,
                total_credits: 1_300,
                debit_balance_total: 1_000,
                credit_balance_total: 1_000,
            })
        );
    }

    #[test]
    fn unknown_account_is_rejected() {
        let ledger_accounts = accounts();
        let postings = [Posting {
            entry_id: 40,
            account_id: 99,
            debit: 100,
            credit: 0,
        }];
        let mut out = [AccountBalance {
            account_id: 0,
            account_type: AccountType::Asset,
            balance: 0,
            debit_total: 0,
            credit_total: 0,
        }; 5];

        assert_eq!(
            account_balances_into(&ledger_accounts, &postings, &mut out),
            Err(AccountingError::UnknownAccount { account_id: 99 })
        );
        assert_eq!(
            trial_balance(&ledger_accounts, &postings),
            Err(AccountingError::UnknownAccount { account_id: 99 })
        );
    }

    #[test]
    fn output_buffer_too_small_is_rejected() {
        let ledger_accounts = accounts();
        let postings = [Posting {
            entry_id: 50,
            account_id: CASH,
            debit: 100,
            credit: 0,
        }];
        let mut out = [AccountBalance {
            account_id: 0,
            account_type: AccountType::Asset,
            balance: 0,
            debit_total: 0,
            credit_total: 0,
        }; 4];

        assert_eq!(
            account_balances_into(&ledger_accounts, &postings, &mut out),
            Err(AccountingError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn journal_entry_range_validation_checks_ids_and_balance() {
        let postings = [
            Posting {
                entry_id: 60,
                account_id: CASH,
                debit: 100,
                credit: 0,
            },
            Posting {
                entry_id: 60,
                account_id: REVENUE,
                debit: 0,
                credit: 100,
            },
        ];
        let entry = JournalEntry {
            id: 60,
            posting_start: 0,
            posting_len: 2,
        };
        assert_eq!(validate_journal_entry(&entry, &postings), Ok(()));
    }

    #[test]
    fn journal_entry_rejects_wrong_posting_id_in_range() {
        let postings = [
            Posting {
                entry_id: 70,
                account_id: CASH,
                debit: 100,
                credit: 0,
            },
            Posting {
                entry_id: 71,
                account_id: REVENUE,
                debit: 0,
                credit: 100,
            },
        ];
        let entry = JournalEntry {
            id: 70,
            posting_start: 0,
            posting_len: 2,
        };
        assert_eq!(
            validate_journal_entry(&entry, &postings),
            Err(AccountingError::InvalidPosting)
        );
    }
}
