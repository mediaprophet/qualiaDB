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
    /// `true` when `debit_normal`.
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

/// Zero-heap conservation error for integer minor-unit journal verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountingConservationError {
    EmptyJournal,
    NegativeAmount { index: usize, amount_cents: i64 },
    UnbalancedJournal { total_debits: i128, total_credits: i128 },
}

/// Zero-heap integer minor-unit double-entry conservation verifier (`AGENTS.md` §0-A).
///
/// Verifies that all debit and credit legs are non-negative, non-empty, and satisfy
/// `sum(debits_cents) == sum(credits_cents) > 0` in `i128` without heap allocation.
#[inline]
pub fn verify_integer_journal_conservation(
    debits_cents: &[i64],
    credits_cents: &[i64],
) -> Result<i128, AccountingConservationError> {
    if debits_cents.is_empty() || credits_cents.is_empty() {
        return Err(AccountingConservationError::EmptyJournal);
    }
    let mut total_debits = 0i128;
    for (index, &d) in debits_cents.iter().enumerate() {
        if d < 0 {
            return Err(AccountingConservationError::NegativeAmount {
                index,
                amount_cents: d,
            });
        }
        total_debits += d as i128;
    }
    let mut total_credits = 0i128;
    for (index, &c) in credits_cents.iter().enumerate() {
        if c < 0 {
            return Err(AccountingConservationError::NegativeAmount {
                index,
                amount_cents: c,
            });
        }
        total_credits += c as i128;
    }
    if total_debits == 0 || total_debits != total_credits {
        return Err(AccountingConservationError::UnbalancedJournal {
            total_debits,
            total_credits,
        });
    }
    Ok(total_debits)
}

/// Zero-heap compilation of a [`Posting`] into a 48-byte [`crate::NQuin`].
#[inline]
#[allow(non_snake_case)]
pub fn compile_journal_Posting_quin(
    posting: &Posting,
    journal_context_hash: u64,
    posted_at_unix: u32,
) -> Result<crate::NQuin, AccountingError> {
    validate_posting_amounts(posting)?;
    let is_debit = posting.debit > 0;
    let amount = if is_debit {
        posting.debit as u64
    } else {
        posting.credit as u64
    };
    let s = posting.entry_id ^ posting.account_id.rotate_left(17);
    let p = if is_debit {
        crate::q_hash("https://ns.webizen.org/econ/debitPostingCents")
    } else {
        crate::q_hash("https://ns.webizen.org/econ/creditPostingCents")
    };
    let o = (0b001u64 << 60) | (amount & 0x0FFF_FFFF_FFFF_FFFF);
    let metadata = (0b01u64 << 61) | ((posted_at_unix as u64) & 0xFFFF_FFFF);
    let parity = crate::NQuin::calculate_parity(s, p, o, journal_context_hash, metadata);
    Ok(crate::NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: journal_context_hash,
        metadata,
        parity,
    })
}

/// Snake-case alias for [`compile_journal_Posting_quin`].
#[inline]
pub fn compile_journal_posting_quin(
    posting: &Posting,
    journal_context_hash: u64,
    posted_at_unix: u32,
) -> Result<crate::NQuin, AccountingError> {
    compile_journal_Posting_quin(posting, journal_context_hash, posted_at_unix)
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
#[path = "accounting_tests.rs"]
mod tests;
