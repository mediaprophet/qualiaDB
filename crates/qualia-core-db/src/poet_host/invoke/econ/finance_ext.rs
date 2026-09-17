//! Finance invoke extensions — currency conversion, multi-sig, ledger.

use super::super::args;
use crate::modalities::{carrier, value_flow};
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Finance.convert_currency` — convert an amount using a rate in micros.
/// `amount` is the source amount; `rate_micros` is target units per source
/// unit in millionths (e.g. 1_500_000 = 1.5x).
pub fn convert_currency(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let amount = args::rec_u64(args, "amount")
        .ok_or_else(|| args::bad(span, "Finance.convert_currency needs amount"))?;
    let rate_micros = args::rec_u64(args, "rate_micros")
        .ok_or_else(|| args::bad(span, "Finance.convert_currency needs rate_micros"))?;
    let converted = value_flow::convert_currency(amount, rate_micros);
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

/// `Finance.ledger_balance` — compute account balances from postings.
/// Takes `accounts` (list of {id, type}) and `postings` (list of {entry_id,
/// account_id, debit, credit}). Returns a list of balances.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn ledger_balance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::accounting::{
        account_balances_into, Account, AccountType, Posting,
    };

    let accounts_val = args::rec(args, "accounts")
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance needs accounts"))?;
    let postings_val = args::rec(args, "postings")
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance needs postings"))?;

    let accounts_list = args::list(accounts_val)
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance accounts must be a list"))?;
    let postings_list = args::list(postings_val)
        .ok_or_else(|| args::bad(span, "Finance.ledger_balance postings must be a list"))?;

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

    let postings: Vec<Posting> = postings_list
        .iter()
        .map(|p| Posting {
            entry_id: args::rec_u64(p, "entry_id").unwrap_or(0),
            account_id: args::rec_u64(p, "account_id").unwrap_or(0),
            debit: args::rec_i64(p, "debit").unwrap_or(0),
            credit: args::rec_i64(p, "credit").unwrap_or(0),
        })
        .collect();

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
            let balances: Vec<Value> = out[..count]
                .iter()
                .map(|b| {
                    let ty_str = match b.account_type {
                        AccountType::Asset => "asset",
                        AccountType::Liability => "liability",
                        AccountType::Equity => "equity",
                        AccountType::Revenue => "revenue",
                        AccountType::Expense => "expense",
                    };
                    args::record([
                        ("account_id", Value::U64(b.account_id)),
                        ("type", Value::String(ty_str.into())),
                        ("balance", Value::I64(b.balance as i64)),
                        ("debit_total", Value::I64(b.debit_total as i64)),
                        ("credit_total", Value::I64(b.credit_total as i64)),
                    ])
                })
                .collect();
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
}
