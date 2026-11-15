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

#[test]
fn integer_journal_conservation_and_posting_quin() {
    assert_eq!(
        verify_integer_journal_conservation(&[60_000, 40_000], &[75_000, 25_000]),
        Ok(100_000)
    );
    assert_eq!(
        verify_integer_journal_conservation(&[50_000], &[49_999]),
        Err(AccountingConservationError::UnbalancedJournal {
            total_debits: 50_000,
            total_credits: 49_999,
        })
    );
    assert_eq!(
        verify_integer_journal_conservation(&[-1], &[1]),
        Err(AccountingConservationError::NegativeAmount {
            index: 0,
            amount_cents: -1,
        })
    );

    let posting = Posting {
        entry_id: 88,
        account_id: CASH,
        debit: 25_000,
        credit: 0,
    };
    let quin = compile_journal_Posting_quin(&posting, 42, 1_700_000_000).unwrap();
    assert!(quin.verify_ecc_parity());
}
