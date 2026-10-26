//! Finance invoke extensions — currency conversion, multi-sig, ledger.

use super::super::args;
use crate::modalities::{carrier, socioeconomics};
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Finance.convert_currency` — convert an amount using a rate in micros.
/// `amount` is the source amount; `rate_micros` is target units per source
/// unit in millionths (e.g. 1_500_000 = 1.5x).
pub fn convert_currency(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let amount = args::rec_u64(args, "amount")
        .ok_or_else(|| args::bad(span, "Finance.convert_currency needs amount"))?;
    let rate_micros = args::rec_u64(args, "rate_micros")
        .ok_or_else(|| args::bad(span, "Finance.convert_currency needs rate_micros"))?;
    let converted = socioeconomics::convert_currency(amount, rate_micros);
    Ok(args::record([
        ("amount", Value::U64(amount)),
        ("rate_micros", Value::U64(rate_micros)),
        ("converted", Value::U64(converted)),
    ]))
}

/// `Finance.multisig_check` — check whether k-of-N multi-sig is satisfied.
/// `valid_signers` is the count of distinct verified signers; `k` is the
/// threshold.
pub fn multisig_check(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let valid = args::rec_u64(args, "valid_signers")
        .ok_or_else(|| args::bad(span, "Finance.multisig_check needs valid_signers"))?;
    let k = args::rec_u64(args, "k")
        .ok_or_else(|| args::bad(span, "Finance.multisig_check needs k"))?;
    let satisfied = carrier::multisig_satisfied(valid as usize, k as usize);
    Ok(args::record([
        ("valid_signers", Value::U64(valid)),
        ("k", Value::U64(k)),
        ("satisfied", Value::Bool(satisfied)),
    ]))
}

/// `Finance.ledger_balance` — compute account balances from double-entry postings.
///
/// Validates per-entry journal balance (`validate_balanced_entry`) and overall
/// `trial_balance` by default unless `allow_unbalanced_projection: true` (or
/// `validate_entries: false`) is explicitly set. Performs checked `i128 -> i64`
/// conversion (`i64::try_from`) so wide totals never silently truncate.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn ledger_balance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::accounting::{
        account_balances_into, trial_balance, validate_balanced_entry, Account, AccountType,
        Posting,
    };

    let accounts_val = args::rec(args, "accounts")
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance needs accounts"))?;
    let postings_val = args::rec(args, "postings")
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance needs postings"))?;

    let accounts_list = args::list(accounts_val)
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance accounts must be a list"))?;
    let postings_list = args::list(postings_val)
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance postings must be a list"))?;

    let allow_unbalanced_projection = args::rec_bool(args, "allow_unbalanced_projection")
        .unwrap_or_else(|| !args::rec_bool(args, "validate_entries").unwrap_or(true));

    let accounts: Vec<Account> = accounts_list
        .iter()
        .map(|a| {
            let id = args::rec_u64(a, "id").unwrap_or(0);
            let ty = match args::rec_str(a, "type").unwrap_or("asset") {
                "liability" => AccountType::Liability,
                "equity" => AccountType::Equity,
                "revenue" => AccountType::Revenue,
                "expense" => AccountType::Expense,
                _ => AccountType::Asset,
            };
            Account {
                id,
                account_type: ty,
            }
        })
        .collect();

    let mut postings = Vec::with_capacity(postings_list.len());
    let mut entry_ids = Vec::new();
    for p in postings_list {
        let entry_id = args::rec_u64(p, "entry_id").unwrap_or(0);
        if !entry_ids.contains(&entry_id) {
            entry_ids.push(entry_id);
        }
        postings.push(Posting {
            entry_id,
            account_id: args::rec_u64(p, "account_id").unwrap_or(0),
            debit: args::rec_i64(p, "debit").unwrap_or(0),
            credit: args::rec_i64(p, "credit").unwrap_or(0),
        });
    }

    let mut entries_balanced = true;
    for &eid in &entry_ids {
        if let Err(e) = validate_balanced_entry(eid, &postings) {
            entries_balanced = false;
            if !allow_unbalanced_projection {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    span,
                    format!(
                        "Finance.ledger_balance: journal entry {eid} failed double-entry validation: {e:?}"
                    ),
                ));
            }
        }
    }

    let trial_ok = match trial_balance(&accounts, &postings) {
        Ok(_) => true,
        Err(e) => {
            if !allow_unbalanced_projection {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    span,
                    format!("Finance.ledger_balance: trial balance failed: {e:?}"),
                ));
            }
            false
        }
    };
    let journal_balanced = !accounts.is_empty() && !postings.is_empty() && entries_balanced && trial_ok;

    let mut out = vec![
        crate::specialized_libs::computational_economics::accounting::AccountBalance {
            account_id: 0,
            account_type: AccountType::Asset,
            balance: 0,
            debit_total: 0,
            credit_total: 0,
        };
        accounts.len()
    ];
    match account_balances_into(&accounts, &postings, &mut out) {
        Ok(count) => {
            let mut balances = Vec::with_capacity(count);
            for b in &out[..count] {
                let ty_str = match b.account_type {
                    AccountType::Asset => "asset",
                    AccountType::Liability => "liability",
                    AccountType::Equity => "equity",
                    AccountType::Revenue => "revenue",
                    AccountType::Expense => "expense",
                };
                let balance_i64 = i64::try_from(b.balance).map_err(|_| {
                    Diagnostic::new(
                        DiagCode::E400,
                        span,
                        format!(
                            "Finance.ledger_balance: account {} balance {} overflows i64",
                            b.account_id, b.balance
                        ),
                    )
                })?;
                let debit_i64 = i64::try_from(b.debit_total).map_err(|_| {
                    Diagnostic::new(
                        DiagCode::E400,
                        span,
                        format!(
                            "Finance.ledger_balance: account {} debit_total {} overflows i64",
                            b.account_id, b.debit_total
                        ),
                    )
                })?;
                let credit_i64 = i64::try_from(b.credit_total).map_err(|_| {
                    Diagnostic::new(
                        DiagCode::E400,
                        span,
                        format!(
                            "Finance.ledger_balance: account {} credit_total {} overflows i64",
                            b.account_id, b.credit_total
                        ),
                    )
                })?;
                balances.push(args::record([
                    ("account_id", Value::U64(b.account_id)),
                    ("type", Value::String(ty_str.into())),
                    ("balance", Value::I64(balance_i64)),
                    ("debit_total", Value::I64(debit_i64)),
                    ("credit_total", Value::I64(credit_i64)),
                    ("balance_i128", Value::String(b.balance.to_string())),
                    ("debit_total_i128", Value::String(b.debit_total.to_string())),
                    ("credit_total_i128", Value::String(b.credit_total.to_string())),
                    ("entries_balanced", Value::Bool(entries_balanced)),
                    ("trial_balance_ok", Value::Bool(trial_ok)),
                    ("journal_balanced", Value::Bool(journal_balanced)),
                    ("projection_only", Value::Bool(!journal_balanced)),
                ]));
            }
            Ok(Value::List(balances))
        }
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Finance.ledger_balance: {e:?}"),
        )),
    }
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
/// `?::ledger_balance`.
pub fn ledger_balance(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Finance.ledger_balance"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn convert_currency_basic() {
        let mut m = BTreeMap::new();
        m.insert("amount".into(), Value::U64(100));
        m.insert("rate_micros".into(), Value::U64(1_500_000)); // 1.5x
        let result = convert_currency(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("converted") {
                Some(Value::U64(v)) => assert_eq!(*v, 150),
                _ => panic!("expected u64 converted"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn multisig_satisfied_check() {
        let mut m = BTreeMap::new();
        m.insert("valid_signers".into(), Value::U64(3));
        m.insert("k".into(), Value::U64(2));
        let result = multisig_check(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("satisfied") {
                Some(Value::Bool(b)) => assert!(*b),
                _ => panic!("expected bool"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn multisig_not_satisfied() {
        let mut m = BTreeMap::new();
        m.insert("valid_signers".into(), Value::U64(1));
        m.insert("k".into(), Value::U64(3));
        let result = multisig_check(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("satisfied") {
                Some(Value::Bool(b)) => assert!(!*b),
                _ => panic!("expected bool"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn ledger_balance_validates_entries_and_checks_wide_totals_lx64b() {
        let accounts = Value::List(vec![
            args::record([("id", Value::U64(1)), ("type", Value::String("asset".into()))]),
            args::record([("id", Value::U64(2)), ("type", Value::String("revenue".into()))]),
        ]);

        // 1. Valid balanced entry succeeds and reports journal_balanced = true
        let balanced_postings = Value::List(vec![
            args::record([
                ("entry_id", Value::U64(100)),
                ("account_id", Value::U64(1)),
                ("debit", Value::I64(2_500)),
                ("credit", Value::I64(0)),
            ]),
            args::record([
                ("entry_id", Value::U64(100)),
                ("account_id", Value::U64(2)),
                ("debit", Value::I64(0)),
                ("credit", Value::I64(2_500)),
            ]),
        ]);
        let ok_args = args::record([
            ("accounts", accounts.clone()),
            ("postings", balanced_postings),
        ]);
        let res = ledger_balance(&ok_args, Span { start: 0, end: 0 }).unwrap();
        let Value::List(rows) = res else {
            panic!("expected list");
        };
        assert_eq!(rows.len(), 2);
        let Value::Record(r0) = &rows[0] else {
            panic!("expected record");
        };
        assert_eq!(r0.get("balance"), Some(&Value::I64(2_500)));
        assert_eq!(r0.get("journal_balanced"), Some(&Value::Bool(true)));
        assert_eq!(r0.get("projection_only"), Some(&Value::Bool(false)));

        // 2. Offsetting unbalanced entries (Entry 1: Debit 500; Entry 2: Credit 500) are rejected by default!
        let offsetting_postings = Value::List(vec![
            args::record([
                ("entry_id", Value::U64(1)),
                ("account_id", Value::U64(1)),
                ("debit", Value::I64(500)),
                ("credit", Value::I64(0)),
            ]),
            args::record([
                ("entry_id", Value::U64(2)),
                ("account_id", Value::U64(2)),
                ("debit", Value::I64(0)),
                ("credit", Value::I64(500)),
            ]),
        ]);
        let bad_args = args::record([
            ("accounts", accounts.clone()),
            ("postings", offsetting_postings.clone()),
        ]);
        assert!(ledger_balance(&bad_args, Span { start: 0, end: 0 }).is_err());

        // 3. Explicit `allow_unbalanced_projection: true` permits projection but marks `journal_balanced: false`
        let proj_args = args::record([
            ("accounts", accounts.clone()),
            ("postings", offsetting_postings),
            ("allow_unbalanced_projection", Value::Bool(true)),
        ]);
        let proj_res = ledger_balance(&proj_args, Span { start: 0, end: 0 }).unwrap();
        let Value::List(proj_rows) = proj_res else {
            panic!("expected list");
        };
        let Value::Record(pr0) = &proj_rows[0] else {
            panic!("expected record");
        };
        assert_eq!(pr0.get("entries_balanced"), Some(&Value::Bool(false)));
        assert_eq!(pr0.get("journal_balanced"), Some(&Value::Bool(false)));
        assert_eq!(pr0.get("projection_only"), Some(&Value::Bool(true)));

        // 4. Out-of-range i128 totals (> i64::MAX) fail closed with DiagCode::E400 instead of truncating with `as i64`
        let overflow_postings = Value::List(vec![
            args::record([
                ("entry_id", Value::U64(10)),
                ("account_id", Value::U64(1)),
                ("debit", Value::I64(i64::MAX)),
                ("credit", Value::I64(0)),
            ]),
            args::record([
                ("entry_id", Value::U64(10)),
                ("account_id", Value::U64(2)),
                ("debit", Value::I64(0)),
                ("credit", Value::I64(i64::MAX)),
            ]),
            args::record([
                ("entry_id", Value::U64(11)),
                ("account_id", Value::U64(1)),
                ("debit", Value::I64(100)),
                ("credit", Value::I64(0)),
            ]),
            args::record([
                ("entry_id", Value::U64(11)),
                ("account_id", Value::U64(2)),
                ("debit", Value::I64(0)),
                ("credit", Value::I64(100)),
            ]),
        ]);
        let ovf_args = args::record([("accounts", accounts), ("postings", overflow_postings)]);
        let ovf_err = ledger_balance(&ovf_args, Span { start: 0, end: 0 })
            .expect_err("i128 total exceeding i64::MAX must fail closed");
        assert_eq!(ovf_err.code, DiagCode::E400);
    }
}
