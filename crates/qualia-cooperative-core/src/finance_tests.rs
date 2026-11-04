use super::*;

fn entry(id: &str, amount: i64, currency: &str, at: u32) -> LedgerEntry {
    LedgerEntry {
        id: id.into(),
        description: format!("entry-{id}"),
        amount_cents: amount,
        currency: currency.into(),
        category: None,
        counterparty: None,
        project_id: None,
        occurred_at_unix: at,
        occurred_at_instant: None,
    }
}

#[test]
fn envelope_kind_and_class() {
    let e = LedgerEntry::new("Groceries", -4200, "aud", 1_700_000_000);
    assert_eq!(e.currency, "AUD");
    let env = build_ledger_entry_envelope(&e, "did:wf:owner", "did:wf:owner", 10, None);
    assert!(env.id.contains(":ledger_entry:"));
    assert_eq!(env.sensitivity, SensitivityClass::Restricted);
}

#[test]
fn merge_dedupes_by_id_add_wins() {
    let a = vec![entry("e1", 1000, "AUD", 1), entry("e2", -500, "AUD", 2)];
    let dup = vec![entry("e1", 1000, "AUD", 1)];
    let merged = merge_ledger(&a, &dup);
    assert_eq!(
        merged.len(),
        2,
        "duplicate id must not create a second entry"
    );
}

#[test]
fn balance_is_invariant_under_duplication_and_reorder() {
    let a = vec![
        entry("e1", 10_000, "AUD", 3),
        entry("e2", -2_500, "AUD", 1),
        entry("e3", 5000, "USD", 2),
    ];
    let incoming = vec![
        entry("e3", 5000, "USD", 2),
        entry("e1", 10_000, "AUD", 3),
        entry("e2", -2_500, "AUD", 1),
        entry("e2", -2_500, "AUD", 1),
    ];
    let merged = merge_ledger(&a, &incoming);
    let bal = derived_balance(&merged);
    assert_eq!(bal.total_entries, 3);
    let aud = bal
        .by_currency
        .iter()
        .find(|c| c.currency == "AUD")
        .unwrap();
    assert_eq!(aud.net_cents, 7_500);
    assert_eq!(aud.entry_count, 2);
    let usd = bal
        .by_currency
        .iter()
        .find(|c| c.currency == "USD")
        .unwrap();
    assert_eq!(usd.net_cents, 5000);

    let bal2 = derived_balance(&merge_ledger(&incoming, &a));
    assert_eq!(bal, bal2);
}

#[test]
fn replaying_incoming_twice_does_not_move_money() {
    let base = vec![entry("e1", 10_000, "AUD", 1)];
    let incoming = vec![entry("e2", -3_000, "AUD", 2)];
    let once = derived_balance(&merge_ledger(&base, &incoming));
    let twice = derived_balance(&merge_ledger(&merge_ledger(&base, &incoming), &incoming));
    assert_eq!(once, twice);
    assert_eq!(once.by_currency[0].net_cents, 7_000);
}

#[test]
fn summary_round_trips_all_ledger_fields_losslessly() {
    let mut e = LedgerEntry::new("Equipment invoice", -180_000, "AUD", 1_700_000_000);
    e.category = Some("infrastructure".into());
    e.counterparty = Some("did:webizen:supplier-1".into());
    e.project_id = Some("proj-solar-coop".into());
    e.occurred_at_instant = Some(InstantBridge::unix(1_700_000_000, 123_456_789));

    let s = ledger_summary(&e);
    let parsed = parse_ledger_summary(&s).expect("summary should parse");
    assert_eq!(parsed, e);
}

#[test]
fn div_round_bankers_half_to_even() {
    assert_eq!(div_round_bankers(5, 2), 2);
    assert_eq!(div_round_bankers(7, 2), 4);
    assert_eq!(div_round_bankers(-5, 2), -2);
    assert_eq!(div_round_bankers(-7, 2), -4);
    assert_eq!(div_round_bankers(10_050, 100), 100);
    assert_eq!(div_round_bankers(10_150, 100), 102);
    assert_eq!(div_round_bankers(100, 0), 0);
}

#[test]
fn journal_entry_balance_and_replay_safe_merge() {
    let mut j1 = JournalEntryRecord::new(
        LedgerBook::ProjectActual,
        "aud",
        "Cloud hosting invoice",
        vec![
            JournalLine {
                account_code: "5100".into(),
                debit_cents: 10_000,
                credit_cents: 0,
                tax_code: Some("GST".into()),
                dimension_project_id: Some("proj-1".into()),
            },
            JournalLine {
                account_code: "2200".into(),
                debit_cents: 1_000,
                credit_cents: 0,
                tax_code: Some("GST".into()),
                dimension_project_id: Some("proj-1".into()),
            },
            JournalLine {
                account_code: "2100".into(),
                debit_cents: 0,
                credit_cents: 11_000,
                tax_code: None,
                dimension_project_id: Some("proj-1".into()),
            },
        ],
        100,
    );
    j1.id = "je-1".into();
    assert!(j1.is_balanced());
    assert!(j1.validate_balance().is_ok());

    let mut unbalanced = j1.clone();
    unbalanced.id = "je-bad".into();
    unbalanced.lines[0].debit_cents = 9_999;
    assert!(!unbalanced.is_balanced());
    assert!(unbalanced.validate_balance().is_err());

    let merged = merge_journal_entries(&[j1.clone()], &[j1.clone(), unbalanced]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].id, "je-1");
}
