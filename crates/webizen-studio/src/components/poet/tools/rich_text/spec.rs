//! Toolbar layout is data. Hosts can swap groups without rewriting the editor.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RichCommand {
    Bold,
    Italic,
    Heading,
    Entity,
    Gazetteer,
}

impl RichCommand {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bold => "B",
            Self::Italic => "I",
            Self::Heading => "H2",
            Self::Entity => "Entity",
            Self::Gazetteer => "Gazetteer",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Heading => "Heading",
            Self::Entity => "Tag ontological entity",
            Self::Gazetteer => "Run document NLP gazetteer",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarGroup {
    pub title: &'static str,
    pub commands: &'static [RichCommand],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarSpec {
    pub groups: &'static [ToolbarGroup],
}

impl ToolbarSpec {
    pub const OFFICE: Self = Self {
        groups: &[
            ToolbarGroup {
                title: "Type",
                commands: &[RichCommand::Bold, RichCommand::Italic, RichCommand::Heading],
            },
            ToolbarGroup {
                title: "Grounding",
                commands: &[RichCommand::Entity, RichCommand::Gazetteer],
            },
        ],
    };
}
