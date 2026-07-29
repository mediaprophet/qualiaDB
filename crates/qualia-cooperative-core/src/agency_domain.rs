//! `AgencyDomain` — the ~17 **domains of agency** a principal may delegate under a supported-agency
//! relationship (see `docs/plans/adr-authority-attestation-guardianship-model.md` §7).
//!
//! (Timothy, 2026-07-03; grounded in the "Agency / Social Book" source and the ADR REFRAME.)
//! This is the *what* of a delegation: which slice of a person's socio-legal life an agent is
//! trusted to help with — an accountant in the **financial** domain, a clinical psychologist in
//! **medical/personal-welfare**, an IT social worker in **it-social-work**, a bot standing in as a
//! declared source of truth for someone isolated in **communication** or **social-welfare**.
//!
//! Two invariants from the source works are load-bearing here:
//! - **Personhood is what gets delegated, never selfhood.** Sixteen of the seventeen domains are
//!   *personhood* (the socio-legal relationship, delegable under an appropriate role). The lone
//!   exception — **reproductive / biometric / genetic** — touches *selfhood*, inherent to the person:
//!   it is the highest bar, never casually delegated, and carried here only so the model can name it
//!   and refuse to treat it like the others. (See `taxonomy::Sphere`.)
//! - **Consequentiality is explicit.** A handful of domains (medical, legal, financial,
//!   reproductive/biometric/genetic, civic-political) carry a `consequential` marker so higher
//!   attestation/threshold requirements can key off it rather than off ad-hoc rules.
//!
//! Modelled as open data in a [`Taxonomy`] so jurisdiction packs, hybrids, and "domain fabrics" can
//! extend the vocabulary without a code change. The seventeen well-known domains and their four
//! grouping categories (welfare, socio-economic, technological, civic) get `const` ids.

use crate::taxonomy::{Taxonomy, TaxonomyTerm};

/// Stable ids for the seeded categories and the seventeen domains (open registry — well-known ones).
pub mod ids {
    // Categories (coarse groupings of domains).
    pub const CATEGORY_WELFARE: &str = "urn:qualia:agency-domain:category:welfare";
    pub const CATEGORY_SOCIO_ECONOMIC: &str = "urn:qualia:agency-domain:category:socio_economic";
    pub const CATEGORY_TECHNOLOGICAL: &str = "urn:qualia:agency-domain:category:technological";
    pub const CATEGORY_CIVIC: &str = "urn:qualia:agency-domain:category:civic";

    // Welfare domains.
    pub const MEDICAL: &str = "urn:qualia:agency-domain:welfare:medical";
    pub const PERSONAL_WELFARE: &str = "urn:qualia:agency-domain:welfare:personal_welfare";
    pub const RESIDENTIAL: &str = "urn:qualia:agency-domain:welfare:residential";
    pub const SUPERVISORY_PROTECTIVE: &str =
        "urn:qualia:agency-domain:welfare:supervisory_protective";
    pub const REPRODUCTIVE_BIOMETRIC_GENETIC: &str =
        "urn:qualia:agency-domain:welfare:reproductive_biometric_genetic";

    // Socio-economic domains.
    pub const FINANCIAL: &str = "urn:qualia:agency-domain:socio_economic:financial";
    pub const LEGAL: &str = "urn:qualia:agency-domain:socio_economic:legal";
    pub const EDUCATION_TRAINING: &str =
        "urn:qualia:agency-domain:socio_economic:education_training";
    pub const SOCIAL_WELFARE: &str = "urn:qualia:agency-domain:socio_economic:social_welfare";

    // Technological domains.
    pub const IT_SOCIAL_WORK: &str = "urn:qualia:agency-domain:technological:it_social_work";
    pub const DIGITAL_IDENTITY: &str = "urn:qualia:agency-domain:technological:digital_identity";
    pub const DATA_PRIVACY_CONSENT: &str =
        "urn:qualia:agency-domain:technological:data_privacy_consent";
    pub const COMMUNICATION: &str = "urn:qualia:agency-domain:technological:communication";
    pub const REPUTATIONAL: &str = "urn:qualia:agency-domain:technological:reputational";
    pub const DIGITAL_LEGACY: &str = "urn:qualia:agency-domain:technological:digital_legacy";
    pub const AI_PROXY: &str = "urn:qualia:agency-domain:technological:ai_proxy";

    // Civic domains.
    pub const CIVIC_POLITICAL: &str = "urn:qualia:agency-domain:civic:civic_political";
}

/// Attribute key: the sphere a domain touches (`"personhood"` for all but reproductive/biometric/genetic).
const ATTR_SPHERE: &str = "sphere";
/// Attribute key: `"true"` when the domain is high-stakes and needs a higher attestation bar.
const ATTR_CONSEQUENTIAL: &str = "consequential";
/// Attribute key: a short comma list of the data classes a delegation in this domain touches.
const ATTR_DATA_CLASSES: &str = "data_classes";

/// Build one domain term: grouped under its category, described, and tagged with sphere / consequential /
/// data-class attributes so ABAC axioms and threshold rules can read them without magic strings.
fn domain(
    id: &str,
    label: &str,
    category: &str,
    description: &str,
    selfhood: bool,
    consequential: bool,
    data_classes: &str,
) -> TaxonomyTerm {
    TaxonomyTerm::new(id, label)
        .in_category(category)
        .described(description)
        .with_attr(
            ATTR_SPHERE,
            if selfhood { "selfhood" } else { "personhood" },
        )
        .with_attr(
            ATTR_CONSEQUENTIAL,
            if consequential { "true" } else { "false" },
        )
        .with_attr(ATTR_DATA_CLASSES, data_classes)
}

/// Build a coarse category term (a grouping of domains).
fn category(id: &str, label: &str) -> TaxonomyTerm {
    TaxonomyTerm::new(id, label).with_attr("kind", "category")
}

/// The seeded domains-of-agency taxonomy: four category terms plus the seventeen domains.
/// Extend with `insert` / `extend_with` for jurisdiction packs, hybrids, or domain fabrics.
pub fn agency_domain_taxonomy() -> Taxonomy {
    use ids::*;
    Taxonomy::from_terms([
        // Categories.
        category(CATEGORY_WELFARE, "Welfare"),
        category(CATEGORY_SOCIO_ECONOMIC, "Socio-economic"),
        category(CATEGORY_TECHNOLOGICAL, "Technological"),
        category(CATEGORY_CIVIC, "Civic"),
        // Welfare domains.
        domain(
            MEDICAL,
            "Medical (healthcare proxy)",
            CATEGORY_WELFARE,
            "Consenting to, coordinating, or refusing medical treatment on the principal's behalf as a healthcare proxy, within their known wishes.",
            false,
            true,
            "diagnosis,medication,treatment",
        ),
        domain(
            PERSONAL_WELFARE,
            "Personal welfare",
            CATEGORY_WELFARE,
            "Day-to-day wellbeing decisions — diet, routine, personal care, social contact — that support the principal without overriding their preferences.",
            false,
            false,
            "care_needs,daily_routine,preferences",
        ),
        domain(
            RESIDENTIAL,
            "Residential",
            CATEGORY_WELFARE,
            "Where and how the principal lives — housing, tenancy, and living arrangements — arranged in line with their choices.",
            false,
            false,
            "address,tenancy,accommodation",
        ),
        domain(
            SUPERVISORY_PROTECTIVE,
            "Supervisory / protective",
            CATEGORY_WELFARE,
            "Oversight to prevent abuse, neglect, or exploitation, with an evidence chain — protective, not custodial capture of the person.",
            false,
            false,
            "safeguarding_alerts,incident_reports",
        ),
        domain(
            REPRODUCTIVE_BIOMETRIC_GENETIC,
            "Reproductive / biometric / genetic",
            CATEGORY_WELFARE,
            "Reproductive autonomy, biometric templates, and genomic data — inherent to the self; the highest bar, never casually delegated.",
            true, // selfhood, not personhood.
            true,
            "genome,biometrics,reproductive_autonomy",
        ),
        // Socio-economic domains.
        domain(
            FINANCIAL,
            "Financial",
            CATEGORY_SOCIO_ECONOMIC,
            "Managing money, accounts, assets, and transactions on the principal's behalf — the accountant / fiduciary case.",
            false,
            true,
            "accounts,transactions,assets,tax",
        ),
        domain(
            LEGAL,
            "Legal",
            CATEGORY_SOCIO_ECONOMIC,
            "Acting in legal matters — representation, contracts, filings, and rights enforcement — under the principal's instruction.",
            false,
            true,
            "contracts,filings,representation",
        ),
        domain(
            EDUCATION_TRAINING,
            "Education & training",
            CATEGORY_SOCIO_ECONOMIC,
            "Enrolment, learning pathways, credentials, and skill development the principal is pursuing or has authorised.",
            false,
            false,
            "enrolment,credentials,progress",
        ),
        domain(
            SOCIAL_WELFARE,
            "Social welfare",
            CATEGORY_SOCIO_ECONOMIC,
            "Benefits, entitlements, and social-support services — applying for and coordinating the safety-net a principal is entitled to.",
            false,
            false,
            "benefits,entitlements,claims",
        ),
        // Technological domains.
        domain(
            IT_SOCIAL_WORK,
            "IT social work",
            CATEGORY_TECHNOLOGICAL,
            "Hands-on help with devices, accounts, and online services for a principal who needs technical support to exercise agency.",
            false,
            false,
            "devices,accounts,support_tickets",
        ),
        domain(
            DIGITAL_IDENTITY,
            "Digital identity",
            CATEGORY_TECHNOLOGICAL,
            "Managing the principal's credentials, DIDs, and identity assertions — who they are online — without impersonating them.",
            false,
            false,
            "dids,credentials,identity_assertions",
        ),
        domain(
            DATA_PRIVACY_CONSENT,
            "Data privacy & consent",
            CATEGORY_TECHNOLOGICAL,
            "Granting, scoping, and revoking consent to share the principal's data — the gatekeeping of who may see what.",
            false,
            false,
            "consent_grants,data_shares,revocations",
        ),
        domain(
            COMMUNICATION,
            "Communication",
            CATEGORY_TECHNOLOGICAL,
            "Sending, receiving, and triaging messages on the principal's behalf — a declared source of truth when they are isolated or overwhelmed.",
            false,
            false,
            "messages,contacts,correspondence",
        ),
        domain(
            REPUTATIONAL,
            "Reputational",
            CATEGORY_TECHNOLOGICAL,
            "Stewarding the principal's public presence and standing — profiles, reviews, and how they are represented to others.",
            false,
            false,
            "profiles,reviews,public_statements",
        ),
        domain(
            DIGITAL_LEGACY,
            "Digital legacy",
            CATEGORY_TECHNOLOGICAL,
            "Handling accounts, data, and assets after death or long-term incapacity — the digital-will / executor case.",
            false,
            false,
            "account_closures,data_bequests,memorialisation",
        ),
        domain(
            AI_PROXY,
            "AI proxy",
            CATEGORY_TECHNOLOGICAL,
            "A software agent executing the principal's pre-declared intents or monitoring streams on their behalf, without overriding their ultimate authority.",
            false,
            false,
            "declared_intents,monitoring_streams,agent_actions",
        ),
        // Civic domains.
        domain(
            CIVIC_POLITICAL,
            "Civic / political",
            CATEGORY_CIVIC,
            "Exercising civic and political agency — voting information, petitions, representations to authorities — amplifying the principal's own voice.",
            false,
            true,
            "voting_info,petitions,representations",
        ),
    ])
}

/// Whether a domain is flagged high-stakes (`consequential = "true"`), needing a higher attestation
/// bar. Reads the seeded attribute; an unknown id (or one lacking the attribute) is `false`.
pub fn is_consequential(tax: &Taxonomy, domain_id: &str) -> bool {
    tax.get(domain_id).and_then(|t| t.attr(ATTR_CONSEQUENTIAL)) == Some("true")
}

#[cfg(test)]
mod tests {
    use super::ids::*;
    use super::*;
    use crate::taxonomy::Sphere;

    /// Every seeded domain id, for exhaustive coverage checks.
    const ALL_DOMAINS: [&str; 17] = [
        MEDICAL,
        PERSONAL_WELFARE,
        RESIDENTIAL,
        SUPERVISORY_PROTECTIVE,
        REPRODUCTIVE_BIOMETRIC_GENETIC,
        FINANCIAL,
        LEGAL,
        EDUCATION_TRAINING,
        SOCIAL_WELFARE,
        IT_SOCIAL_WORK,
        DIGITAL_IDENTITY,
        DATA_PRIVACY_CONSENT,
        COMMUNICATION,
        REPUTATIONAL,
        DIGITAL_LEGACY,
        AI_PROXY,
        CIVIC_POLITICAL,
    ];

    #[test]
    fn exactly_seventeen_domains_across_four_categories() {
        let tax = agency_domain_taxonomy();
        let welfare = tax.in_category(CATEGORY_WELFARE).len();
        let socio = tax.in_category(CATEGORY_SOCIO_ECONOMIC).len();
        let tech = tax.in_category(CATEGORY_TECHNOLOGICAL).len();
        let civic = tax.in_category(CATEGORY_CIVIC).len();
        assert_eq!(welfare, 5, "welfare should hold 5 domains");
        assert_eq!(socio, 4, "socio-economic should hold 4 domains");
        assert_eq!(tech, 7, "technological should hold 7 domains");
        assert_eq!(civic, 1, "civic should hold 1 domain");
        assert_eq!(welfare + socio + tech + civic, 17, "17 domains total");
        // Four categories + seventeen domains = twenty-one terms.
        assert_eq!(tax.len(), 21);
    }

    #[test]
    fn every_named_domain_id_is_present_and_categorised() {
        let tax = agency_domain_taxonomy();
        for id in ALL_DOMAINS {
            let term = tax
                .get(id)
                .unwrap_or_else(|| panic!("missing domain: {id}"));
            assert!(
                term.category.is_some(),
                "domain {id} must sit in a category"
            );
            assert!(
                !term.description.is_empty(),
                "domain {id} must have a description"
            );
            // Every domain names at least one data class.
            assert!(
                !term.attr_list(ATTR_DATA_CLASSES).is_empty(),
                "domain {id} must list data classes"
            );
        }
    }

    #[test]
    fn exactly_five_consequential_domains_and_they_are_the_right_ones() {
        let tax = agency_domain_taxonomy();
        let expected_consequential = [
            MEDICAL,
            LEGAL,
            FINANCIAL,
            REPRODUCTIVE_BIOMETRIC_GENETIC,
            CIVIC_POLITICAL,
        ];
        let consequential: Vec<&str> = ALL_DOMAINS
            .iter()
            .copied()
            .filter(|id| is_consequential(&tax, id))
            .collect();
        assert_eq!(
            consequential.len(),
            5,
            "exactly five domains are consequential, got {consequential:?}"
        );
        for id in expected_consequential {
            assert!(is_consequential(&tax, id), "{id} should be consequential");
        }
        // And nothing else is.
        for id in ALL_DOMAINS {
            let should = expected_consequential.contains(&id);
            assert_eq!(
                is_consequential(&tax, id),
                should,
                "consequential flag wrong for {id}"
            );
        }
    }

    #[test]
    fn reproductive_biometric_genetic_is_selfhood_and_all_others_are_personhood() {
        let tax = agency_domain_taxonomy();
        for id in ALL_DOMAINS {
            let term = tax.get(id).unwrap();
            if id == REPRODUCTIVE_BIOMETRIC_GENETIC {
                assert_eq!(
                    term.sphere(),
                    Sphere::Selfhood,
                    "reproductive/biometric/genetic touches selfhood"
                );
            } else {
                assert_eq!(
                    term.sphere(),
                    Sphere::Personhood,
                    "{id} should be a personhood domain"
                );
            }
        }
    }

    #[test]
    fn is_consequential_reads_the_attribute() {
        let tax = agency_domain_taxonomy();
        assert!(is_consequential(&tax, MEDICAL));
        assert!(!is_consequential(&tax, COMMUNICATION));
    }

    #[test]
    fn is_consequential_is_false_for_unknown_id() {
        let tax = agency_domain_taxonomy();
        assert!(!is_consequential(
            &tax,
            "urn:qualia:agency-domain:welfare:does-not-exist"
        ));
        assert!(!is_consequential(&tax, ""));
    }

    #[test]
    fn category_terms_are_not_counted_as_domains() {
        let tax = agency_domain_taxonomy();
        // A category term carries no consequential attribute → is_consequential is false,
        // and it is not a member of any category itself (top-level).
        assert!(!is_consequential(&tax, CATEGORY_WELFARE));
        assert!(tax.get(CATEGORY_WELFARE).unwrap().category.is_none());
        assert_eq!(
            tax.get(CATEGORY_WELFARE).unwrap().attr("kind"),
            Some("category")
        );
    }

    #[test]
    fn medical_data_classes_parse() {
        let tax = agency_domain_taxonomy();
        let classes = tax.get(MEDICAL).unwrap().attr_list(ATTR_DATA_CLASSES);
        assert_eq!(classes, vec!["diagnosis", "medication", "treatment"]);
    }

    #[test]
    fn taxonomy_round_trips_through_json() {
        let tax = agency_domain_taxonomy();
        let json = serde_json::to_string(&tax).unwrap();
        let back: Taxonomy = serde_json::from_str(&json).unwrap();
        assert_eq!(tax, back);
        assert!(back.contains(AI_PROXY));
    }
}
