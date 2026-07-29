#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RelationsSection {
    #[default]
    Inbox,
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

pub const ALL_SECTIONS: [RelationsSection; 8] = [
    RelationsSection::Inbox,
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
    fn naturalised_information_architecture_has_six_stable_destinations() {
        assert_eq!(
            ALL_SECTIONS
                .iter()
                .filter(|section| !section.advanced_only())
                .count(),
            6
        );
    }

    #[test]
    fn topology_and_existing_tools_are_advanced_only() {
        assert!(RelationsSection::Topology.advanced_only());
        assert!(RelationsSection::ExistingTools.advanced_only());
    }
}
