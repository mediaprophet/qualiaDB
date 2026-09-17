#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RelationsSection {
    #[default]
    Inbox,
    Mail,
    People,
    Groups,
    Requests,
    Reception,
    Agreements,
    Topology,
    ExistingTools,
}

impl RelationsSection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Mail => "Mail",
            Self::People => "People",
            Self::Groups => "Groups & commons",
            Self::Requests => "Requests",
            Self::Reception => "Reception",
            Self::Agreements => "Agreements",
            Self::Topology => "Technical route",
            Self::ExistingTools => "Existing tools",
        }
    }

    pub const fn advanced_only(self) -> bool {
        matches!(self, Self::Topology | Self::ExistingTools)
    }
}

/// Deep-link / palette / Keep handoff for Talk tabs.
pub fn section_from_talk_tab(tab: &str, advanced: bool) -> RelationsSection {
    match tab {
        "people" | "directory" | "dir" | "contacts" | "addressbook" | "address-book"
        | "address book" | "rolodex" => RelationsSection::People,
        "projects" => RelationsSection::Groups,
        "reception" => RelationsSection::Reception,
        "mail" | "email" => RelationsSection::Mail,
        "requests" => RelationsSection::Requests,
        "agreements" => RelationsSection::Agreements,
        "topology" if advanced => RelationsSection::Topology,
        _ => RelationsSection::Inbox,
    }
}

pub const ALL_SECTIONS: [RelationsSection; 9] = [
    RelationsSection::Inbox,
    RelationsSection::Mail,
    RelationsSection::People,
    RelationsSection::Groups,
    RelationsSection::Requests,
    RelationsSection::Reception,
    RelationsSection::Agreements,
    RelationsSection::Topology,
    RelationsSection::ExistingTools,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naturalised_information_architecture_has_seven_stable_destinations() {
        assert_eq!(
            ALL_SECTIONS
                .iter()
                .filter(|section| !section.advanced_only())
                .count(),
            7
        );
        assert!(ALL_SECTIONS.contains(&RelationsSection::Mail));
    }

    #[test]
    fn topology_and_existing_tools_are_advanced_only() {
        assert!(RelationsSection::Topology.advanced_only());
        assert!(RelationsSection::ExistingTools.advanced_only());
    }

    #[test]
    fn mail_deep_link_opens_daily_inbox_not_domains_admin() {
        assert_eq!(section_from_talk_tab("mail", false), RelationsSection::Mail);
        assert_eq!(section_from_talk_tab("email", true), RelationsSection::Mail);
        assert_eq!(section_from_talk_tab("dir", false), RelationsSection::People);
        assert_eq!(
            section_from_talk_tab("directory", false),
            RelationsSection::People
        );
        assert_eq!(
            section_from_talk_tab("address book", false),
            RelationsSection::People
        );
        assert_eq!(
            section_from_talk_tab("reception", false),
            RelationsSection::Reception
        );
        assert_ne!(
            section_from_talk_tab("mail", false),
            RelationsSection::Reception
        );
    }
}
