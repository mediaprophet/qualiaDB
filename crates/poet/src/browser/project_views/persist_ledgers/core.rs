//! Simple Project views that persist one COP family each.

use web_sys::{Document, Element};

use super::super::super::cop_records::CopField;
use super::super::persist::ledger;

pub fn build_issues_view(document: &Document) -> Element {
    ledger(
        document,
        "project_issue",
        "Live project issues. Save a record to report; sample tickets are not shown.",
        &[
            CopField {
                key: "type",
                placeholder: "Type (bug|incident|report)",
            },
            CopField {
                key: "severity",
                placeholder: "Severity (critical|high|medium|low)",
            },
            CopField {
                key: "status",
                placeholder: "Status (open|resolved)",
            },
            CopField {
                key: "version",
                placeholder: "Version",
            },
            CopField {
                key: "reproducibility",
                placeholder: "Reproducibility",
            },
        ],
    )
}

pub fn build_wiki_view(document: &Document) -> Element {
    ledger(
        document,
        "project_wiki",
        "Wiki pages persist as COP records with category and summary. There is no fabricated page tree.",
        &[
            CopField {
                key: "category",
                placeholder: "Category",
            },
            CopField {
                key: "author",
                placeholder: "Author DID",
            },
            CopField {
                key: "version",
                placeholder: "Version label",
            },
            CopField {
                key: "summary",
                placeholder: "Summary (≤1024 bytes)",
            },
        ],
    )
}

pub fn build_knowledge_base_view(document: &Document) -> Element {
    ledger(
        document,
        "project_knowledge",
        "Knowledge entries persist independently of wiki pages.",
        &[
            CopField {
                key: "topic",
                placeholder: "Topic",
            },
            CopField {
                key: "source",
                placeholder: "Source",
            },
            CopField {
                key: "summary",
                placeholder: "Summary",
            },
        ],
    )
}

pub fn build_doc_mgmt_view(document: &Document) -> Element {
    ledger(
        document,
        "project_document",
        "Document registry records. Binary attachments are not stored in this ledger.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (spec|minutes|report)",
            },
            CopField {
                key: "license",
                placeholder: "License",
            },
            CopField {
                key: "uri",
                placeholder: "URI or path",
            },
        ],
    )
}

pub fn build_deliverable_view(document: &Document) -> Element {
    ledger(
        document,
        "project_deliverable",
        "Deliverables persist with status and due date.",
        &[
            CopField {
                key: "status",
                placeholder: "Status (draft|review|accepted)",
            },
            CopField {
                key: "due",
                placeholder: "Due (YYYY-MM-DD)",
            },
            CopField {
                key: "owner",
                placeholder: "Owner DID",
            },
        ],
    )
}

pub fn build_roadmap_view(document: &Document) -> Element {
    ledger(
        document,
        "project_milestone",
        "Roadmap milestones are the same family as dashboard milestones.",
        &[
            CopField {
                key: "date",
                placeholder: "Date (YYYY-MM-DD)",
            },
            CopField {
                key: "status",
                placeholder: "Status (on_track|at_risk|delayed|not_started)",
            },
            CopField {
                key: "phase",
                placeholder: "Phase",
            },
        ],
    )
}

pub fn build_risk_view(document: &Document) -> Element {
    ledger(
        document,
        "project_risk",
        "Risk register. Likelihood and impact are stored fields, not scored sample risks.",
        &[
            CopField {
                key: "likelihood",
                placeholder: "Likelihood (low|medium|high)",
            },
            CopField {
                key: "impact",
                placeholder: "Impact (low|medium|high)",
            },
            CopField {
                key: "status",
                placeholder: "Status (open|mitigated)",
            },
            CopField {
                key: "owner",
                placeholder: "Owner DID",
            },
        ],
    )
}

pub fn build_budget_view(document: &Document) -> Element {
    ledger(
        document,
        "project_budget",
        "Budget lines persist as COP records. Totals are not invented.",
        &[
            CopField {
                key: "amount",
                placeholder: "Amount",
            },
            CopField {
                key: "currency",
                placeholder: "Currency",
            },
            CopField {
                key: "category",
                placeholder: "Category",
            },
            CopField {
                key: "status",
                placeholder: "Status (planned|spent)",
            },
        ],
    )
}

pub fn build_cost_base_view(document: &Document) -> Element {
    ledger(
        document,
        "project_cost",
        "Cost-base rows (unit cost / capacity). Distinct from budget lines.",
        &[
            CopField {
                key: "unit",
                placeholder: "Unit",
            },
            CopField {
                key: "rate",
                placeholder: "Rate",
            },
            CopField {
                key: "capacity",
                placeholder: "Capacity",
            },
        ],
    )
}

pub fn build_asset_mgr_view(document: &Document) -> Element {
    ledger(
        document,
        "project_asset",
        "Asset registry with license and provenance fields.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind",
            },
            CopField {
                key: "license",
                placeholder: "License",
            },
            CopField {
                key: "provenance",
                placeholder: "Provenance",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
        ],
    )
}

pub fn build_discussion_view(document: &Document) -> Element {
    ledger(
        document,
        "project_discussion",
        "Discussion threads persist as records. There is no fabricated comment graph.",
        &[
            CopField {
                key: "thread",
                placeholder: "Thread id or topic",
            },
            CopField {
                key: "author",
                placeholder: "Author DID",
            },
            CopField {
                key: "body",
                placeholder: "Body (≤1024 bytes)",
            },
        ],
    )
}

pub fn build_governance_view(document: &Document) -> Element {
    ledger(
        document,
        "project_governance",
        "Project governance configuration records (policy, quorum, instrument).",
        &[
            CopField {
                key: "instrument",
                placeholder: "Instrument (COP-R4|…)",
            },
            CopField {
                key: "quorum",
                placeholder: "Quorum",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
    )
}

pub fn build_voting_view(document: &Document) -> Element {
    ledger(
        document,
        "project_vote",
        "Votes persist as records. Tallies are not computed until a consensus invoke is bound.",
        &[
            CopField {
                key: "motion",
                placeholder: "Motion",
            },
            CopField {
                key: "choice",
                placeholder: "Choice (aye|nay|abstain)",
            },
            CopField {
                key: "voter",
                placeholder: "Voter DID",
            },
        ],
    )
}

pub fn build_awards_view(document: &Document) -> Element {
    ledger(
        document,
        "project_award",
        "Awards persist as records. Token minting is not performed here.",
        &[
            CopField {
                key: "recipient",
                placeholder: "Recipient DID",
            },
            CopField {
                key: "reason",
                placeholder: "Reason",
            },
            CopField {
                key: "status",
                placeholder: "Status (proposed|approved)",
            },
        ],
    )
}

pub fn build_bounties_view(document: &Document) -> Element {
    ledger(
        document,
        "project_bounty",
        "Bounties persist as records. Escrow/wallet settlement is not bound.",
        &[
            CopField {
                key: "amount",
                placeholder: "Amount",
            },
            CopField {
                key: "currency",
                placeholder: "Currency",
            },
            CopField {
                key: "status",
                placeholder: "Status (open|claimed|paid)",
            },
            CopField {
                key: "assignee",
                placeholder: "Assignee DID",
            },
        ],
    )
}
