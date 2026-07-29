//! Studio academic QApp catalogue — seeded into the hypermedia Library **Software** section.
//!
//! These entries inventory the early/stub liberal-arts QApps under
//! `webizen-studio/src/components/*_qapp.rs`. They are **catalogued and categorised**
//! for faceted browse — not claimed as fully implemented apps.

use super::hypermedia_store::{CommonsVisibility, HypermediaStore, LibraryEntry, LibrarySection};
use serde::{Deserialize, Serialize};

/// Stable media type for studio-pane QApps in the Software shelf.
pub const QAPP_MEDIA_TYPE: &str = "application/x-webizen-qapp";

/// One catalogue row (id = module stem without `_qapp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QappCatalogEntry {
    pub id: &'static str,
    pub title: &'static str,
    /// Domain category slug (faceted browse).
    pub category: &'static str,
}

impl QappCatalogEntry {
    pub fn asset_uri(self) -> String {
        format!("qapp://studio/{}", self.id)
    }

    pub fn display_title(self) -> String {
        self.title.to_string()
    }
}

/// Full inventory of studio academic QApps with domain categories.
pub const STUDIO_QAPP_CATALOG: &[QappCatalogEntry] = &[
    QappCatalogEntry {
        id: "african_american_studies",
        title: "African American Studies",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "anthropology",
        title: "Anthropology",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "archaeology",
        title: "Archaeology",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "area_and_regional_studies",
        title: "Area And Regional Studies",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "cultural_studies",
        title: "Cultural Studies",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "economics",
        title: "Economics",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "gender_and_sexuality_studies",
        title: "Gender And Sexuality Studies",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "geography_human_geography",
        title: "Geography Human Geography",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "history",
        title: "History",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "international_relations",
        title: "International Relations",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "political_science",
        title: "Political Science",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "psychology",
        title: "Psychology",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "sociology",
        title: "Sociology",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "art_history",
        title: "Art History",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "classics",
        title: "Classics",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "comparative_literature",
        title: "Comparative Literature",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "creative_writing",
        title: "Creative Writing",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "dance",
        title: "Dance",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "english_language_and_literature",
        title: "English Language And Literature",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "ethics",
        title: "Ethics",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "film_and_media_studies",
        title: "Film And Media Studies",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "foreign_languages_and_literatures",
        title: "Foreign Languages And Literatures",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "linguistics",
        title: "Linguistics",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "music_history",
        title: "Music History",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "music_performance",
        title: "Music Performance",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "music_theory",
        title: "Music Theory",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "philosophy",
        title: "Philosophy",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "religion_and_theology",
        title: "Religion And Theology",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "studio_art",
        title: "Studio Art",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "theater_and_drama",
        title: "Theater And Drama",
        category: "humanities",
    },
    QappCatalogEntry {
        id: "astronomy",
        title: "Astronomy",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "astrophysics",
        title: "Astrophysics",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "biology",
        title: "Biology",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "botany",
        title: "Botany",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "chemistry",
        title: "Chemistry",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "earth_science",
        title: "Earth Science",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "ecology",
        title: "Ecology",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "environmental_science",
        title: "Environmental Science",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "evolutionary_biology",
        title: "Evolutionary Biology",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "geology",
        title: "Geology",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "neuroscience",
        title: "Neuroscience",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "oceanography",
        title: "Oceanography",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "physics",
        title: "Physics",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "zoology",
        title: "Zoology",
        category: "natural-sciences",
    },
    QappCatalogEntry {
        id: "computer_science",
        title: "Computer Science",
        category: "formal-sciences",
    },
    QappCatalogEntry {
        id: "logic",
        title: "Logic",
        category: "formal-sciences",
    },
    QappCatalogEntry {
        id: "mathematics",
        title: "Mathematics",
        category: "formal-sciences",
    },
    QappCatalogEntry {
        id: "statistics",
        title: "Statistics",
        category: "formal-sciences",
    },
    QappCatalogEntry {
        id: "american_studies",
        title: "American Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "asian_studies",
        title: "Asian Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "cognitive_science",
        title: "Cognitive Science",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "communication_studies",
        title: "Communication Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "european_studies",
        title: "European Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "folklore_and_mythology",
        title: "Folklore And Mythology",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "global_studies",
        title: "Global Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "history_of_science_and_medicine",
        title: "History Of Science And Medicine",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "indigenous_and_native_american_studies",
        title: "Indigenous And Native American Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "jewish_studies",
        title: "Jewish Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "latin_american_studies",
        title: "Latin American Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "medieval_and_renaissance_studies",
        title: "Medieval And Renaissance Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "middle_eastern_studies",
        title: "Middle Eastern Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "peace_and_conflict_studies",
        title: "Peace And Conflict Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "rhetoric_and_composition",
        title: "Rhetoric And Composition",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "science_technology_and_society_sts",
        title: "Science Technology And Society Sts",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "urban_studies",
        title: "Urban Studies",
        category: "area-studies",
    },
    QappCatalogEntry {
        id: "criminology_and_criminal_justice",
        title: "Criminology And Criminal Justice",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "education_studies",
        title: "Education Studies",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "journalism",
        title: "Journalism",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "library_and_information_science",
        title: "Library And Information Science",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "museum_studies",
        title: "Museum Studies",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "public_health",
        title: "Public Health",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "public_policy",
        title: "Public Policy",
        category: "applied-liberal-arts",
    },
    QappCatalogEntry {
        id: "african_studies",
        title: "African Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "celtic_studies",
        title: "Celtic Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "chicano_and_latino_studies",
        title: "Chicano And Latino Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "digital_humanities",
        title: "Digital Humanities",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "disability_studies",
        title: "Disability Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "environmental_humanities",
        title: "Environmental Humanities",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "ethnomusicology",
        title: "Ethnomusicology",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "food_studies",
        title: "Food Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "game_studies",
        title: "Game Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "human_rights_studies",
        title: "Human Rights Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "medical_humanities",
        title: "Medical Humanities",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "oceanic_and_pacific_island_studies",
        title: "Oceanic And Pacific Island Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "postcolonial_studies",
        title: "Postcolonial Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "poverty_and_inequality_studies",
        title: "Poverty And Inequality Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "queer_studies",
        title: "Queer Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "scandinavian_studies",
        title: "Scandinavian Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "slavic_studies",
        title: "Slavic Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "sound_studies",
        title: "Sound Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "sustainability_studies",
        title: "Sustainability Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "translation_studies",
        title: "Translation Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "visual_and_critical_studies",
        title: "Visual And Critical Studies",
        category: "emerging-interdisciplinary",
    },
    QappCatalogEntry {
        id: "astrobiology",
        title: "Astrobiology",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "behavioral_economics",
        title: "Behavioral Economics",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "biomathematics",
        title: "Biomathematics",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "biophysics",
        title: "Biophysics",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "demography_and_population_studies",
        title: "Demography And Population Studies",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "geophysics",
        title: "Geophysics",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "planetary_science",
        title: "Planetary Science",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "political_economy",
        title: "Political Economy",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "social_psychology",
        title: "Social Psychology",
        category: "social-sciences",
    },
    QappCatalogEntry {
        id: "art_conservation",
        title: "Art Conservation",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "arts_management_and_administration",
        title: "Arts Management And Administration",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "bioethics",
        title: "Bioethics",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "curatorial_studies",
        title: "Curatorial Studies",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "leadership_studies",
        title: "Leadership Studies",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "legal_studies",
        title: "Legal Studies",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "social_work",
        title: "Social Work",
        category: "pre-professional",
    },
    QappCatalogEntry {
        id: "ancient_near_eastern_studies",
        title: "Ancient Near Eastern Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "appalachian_studies",
        title: "Appalachian Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "arctic_studies",
        title: "Arctic Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "balkan_studies",
        title: "Balkan Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "caribbean_studies",
        title: "Caribbean Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "central_asian_studies",
        title: "Central Asian Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "egyptology",
        title: "Egyptology",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "francophone_studies",
        title: "Francophone Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "germanic_languages_and_literatures",
        title: "Germanic Languages And Literatures",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "hispanic_and_luso_brazilian_studies",
        title: "Hispanic And Luso Brazilian Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "philology",
        title: "Philology",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "romance_languages_and_literatures",
        title: "Romance Languages And Literatures",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "south_asian_studies",
        title: "South Asian Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "southeast_asian_studies",
        title: "Southeast Asian Studies",
        category: "language-regional",
    },
    QappCatalogEntry {
        id: "animation_and_digital_arts",
        title: "Animation And Digital Arts",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "book_arts_and_papermaking",
        title: "Book Arts And Papermaking",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "ceramics",
        title: "Ceramics",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "cinematography",
        title: "Cinematography",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "dramaturgy",
        title: "Dramaturgy",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "musicology",
        title: "Musicology",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "photography",
        title: "Photography",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "playwriting",
        title: "Playwriting",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "printmaking",
        title: "Printmaking",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "screenwriting",
        title: "Screenwriting",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "sculpture",
        title: "Sculpture",
        category: "arts-performance",
    },
    QappCatalogEntry {
        id: "applied_linguistics",
        title: "Applied Linguistics",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "family_studies",
        title: "Family Studies",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "gerontology",
        title: "Gerontology",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "penology",
        title: "Penology",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "psycholinguistics",
        title: "Psycholinguistics",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "social_and_cultural_analysis",
        title: "Social And Cultural Analysis",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "sociolinguistics",
        title: "Sociolinguistics",
        category: "advanced-subdisciplines",
    },
    QappCatalogEntry {
        id: "atmospheric_science",
        title: "Atmospheric Science",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "behavioral_ecology",
        title: "Behavioral Ecology",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "kinesiology_and_movement_studies",
        title: "Kinesiology And Movement Studies",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "marine_biology",
        title: "Marine Biology",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "materials_science",
        title: "Materials Science",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "meteorology",
        title: "Meteorology",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "mycology",
        title: "Mycology",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "paleontology",
        title: "Paleontology",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "sports_studies",
        title: "Sports Studies",
        category: "niche-sciences",
    },
    QappCatalogEntry {
        id: "aesthetics",
        title: "Aesthetics",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "epistemology",
        title: "Epistemology",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "metaphysics",
        title: "Metaphysics",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "phenomenology",
        title: "Phenomenology",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "philosophy_of_mind",
        title: "Philosophy Of Mind",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "philosophy_of_religion",
        title: "Philosophy Of Religion",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "philosophy_of_science",
        title: "Philosophy Of Science",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "social_and_political_philosophy",
        title: "Social And Political Philosophy",
        category: "philosophy-theory",
    },
    QappCatalogEntry {
        id: "biblical_studies",
        title: "Biblical Studies",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "buddhist_studies",
        title: "Buddhist Studies",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "canon_law",
        title: "Canon Law",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "hindu_studies",
        title: "Hindu Studies",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "islamic_studies",
        title: "Islamic Studies",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "missiology",
        title: "Missiology",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "patristics",
        title: "Patristics",
        category: "religious-studies",
    },
    QappCatalogEntry {
        id: "arthurian_studies",
        title: "Arthurian Studies",
        category: "literary-media",
    },
    QappCatalogEntry {
        id: "comics_and_graphic_novel_studies",
        title: "Comics And Graphic Novel Studies",
        category: "literary-media",
    },
    QappCatalogEntry {
        id: "fan_studies",
        title: "Fan Studies",
        category: "literary-media",
    },
    QappCatalogEntry {
        id: "poetry_and_poetics",
        title: "Poetry And Poetics",
        category: "literary-media",
    },
    QappCatalogEntry {
        id: "science_fiction_and_fantasy_studies",
        title: "Science Fiction And Fantasy Studies",
        category: "literary-media",
    },
    QappCatalogEntry {
        id: "utopian_studies",
        title: "Utopian Studies",
        category: "literary-media",
    },
    QappCatalogEntry {
        id: "historical_linguistics",
        title: "Historical Linguistics",
        category: "linguistics-semiotics",
    },
    QappCatalogEntry {
        id: "morphology",
        title: "Morphology",
        category: "linguistics-semiotics",
    },
    QappCatalogEntry {
        id: "pragmatics",
        title: "Pragmatics",
        category: "linguistics-semiotics",
    },
    QappCatalogEntry {
        id: "semantics",
        title: "Semantics",
        category: "linguistics-semiotics",
    },
    QappCatalogEntry {
        id: "semiotics",
        title: "Semiotics",
        category: "linguistics-semiotics",
    },
    QappCatalogEntry {
        id: "syntax",
        title: "Syntax",
        category: "linguistics-semiotics",
    },
    QappCatalogEntry {
        id: "disaster_studies",
        title: "Disaster Studies",
        category: "intersectional-applied",
    },
    QappCatalogEntry {
        id: "futures_studies_and_foresight",
        title: "Futures Studies And Foresight",
        category: "intersectional-applied",
    },
    QappCatalogEntry {
        id: "leisure_studies",
        title: "Leisure Studies",
        category: "intersectional-applied",
    },
    QappCatalogEntry {
        id: "philanthropy_and_nonprofit_studies",
        title: "Philanthropy And Nonprofit Studies",
        category: "intersectional-applied",
    },
    QappCatalogEntry {
        id: "architectural_history",
        title: "Architectural History",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "childrens_literature",
        title: "Childrens Literature",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "history_of_art_and_architecture",
        title: "History Of Art And Architecture",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "intellectual_history",
        title: "Intellectual History",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "maritime_history",
        title: "Maritime History",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "military_history",
        title: "Military History",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "oral_history",
        title: "Oral History",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "paleography",
        title: "Paleography",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "public_history",
        title: "Public History",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "textual_criticism",
        title: "Textual Criticism",
        category: "historical-textual",
    },
    QappCatalogEntry {
        id: "animal_studies_human_animal_studies",
        title: "Animal Studies Human Animal Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "body_studies",
        title: "Body Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "critical_race_and_ethnic_studies",
        title: "Critical Race And Ethnic Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "critical_theory",
        title: "Critical Theory",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "deaf_studies",
        title: "Deaf Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "diaspora_studies",
        title: "Diaspora Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "fat_studies",
        title: "Fat Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "indigenous_language_revitalization",
        title: "Indigenous Language Revitalization",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "material_culture_studies",
        title: "Material Culture Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "memory_studies",
        title: "Memory Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "whiteness_studies",
        title: "Whiteness Studies",
        category: "critical-cultural",
    },
    QappCatalogEntry {
        id: "biogeochemistry",
        title: "Biogeochemistry",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "bioinformatics",
        title: "Bioinformatics",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "chemical_physics",
        title: "Chemical Physics",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "computational_linguistics",
        title: "Computational Linguistics",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "cryptography",
        title: "Cryptography",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "environmental_chemistry",
        title: "Environmental Chemistry",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "geochemistry",
        title: "Geochemistry",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "mathematical_biology",
        title: "Mathematical Biology",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "mathematical_economics",
        title: "Mathematical Economics",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "systems_biology",
        title: "Systems Biology",
        category: "interdisciplinary-stem",
    },
    QappCatalogEntry {
        id: "architectural_studies",
        title: "Architectural Studies",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "cyberculture_studies",
        title: "Cyberculture Studies",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "environmental_design",
        title: "Environmental Design",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "landscape_studies",
        title: "Landscape Studies",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "media_ecology",
        title: "Media Ecology",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "spatial_data_science",
        title: "Spatial Data Science",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "urban_ecology",
        title: "Urban Ecology",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "urban_planning_and_design",
        title: "Urban Planning And Design",
        category: "design-spatial",
    },
    QappCatalogEntry {
        id: "affect_theory",
        title: "Affect Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "biopolitics",
        title: "Biopolitics",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "capital_studies",
        title: "Capital Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "critical_disability_studies",
        title: "Critical Disability Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "critical_film_studies",
        title: "Critical Film Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "critical_gentrification_studies",
        title: "Critical Gentrification Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "cultural_ecology",
        title: "Cultural Ecology",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "decolonial_studies",
        title: "Decolonial Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "eco_critical_theory",
        title: "Eco Critical Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "eco_feminism",
        title: "Eco Feminism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "eco_queer_theory",
        title: "Eco Queer Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "environmental_justice",
        title: "Environmental Justice",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "film_philosophy",
        title: "Film Philosophy",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "gender_studies",
        title: "Gender Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "global_critical_studies",
        title: "Global Critical Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "grassroots_studies",
        title: "Grassroots Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "grief_studies",
        title: "Grief Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "haunted_humanities",
        title: "Haunted Humanities",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "hermeneutics",
        title: "Hermeneutics",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "historiography",
        title: "Historiography",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "ideology_critique",
        title: "Ideology Critique",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "indigenous_feminisms",
        title: "Indigenous Feminisms",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "integral_studies",
        title: "Integral Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "intermedia_studies",
        title: "Intermedia Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "landscape_phenomenology",
        title: "Landscape Phenomenology",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "liberation_studies",
        title: "Liberation Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "literature_and_law",
        title: "Literature And Law",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "materialist_aesthetics",
        title: "Materialist Aesthetics",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "media_theory",
        title: "Media Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "metamodernism",
        title: "Metamodernism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "post_critical_pedagogy",
        title: "Post Critical Pedagogy",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "posthumanities",
        title: "Posthumanities",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "poststructuralism",
        title: "Poststructuralism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "psychoanalysis",
        title: "Psychoanalysis",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "queer_cinema_studies",
        title: "Queer Cinema Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "queer_theory",
        title: "Queer Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "race_critical_theory",
        title: "Race Critical Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "race_studies",
        title: "Race Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "race_theory",
        title: "Race Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "radical_media_studies",
        title: "Radical Media Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "regionalism",
        title: "Regionalism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "revisionist_critical_theory",
        title: "Revisionist Critical Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "rural_studies",
        title: "Rural Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "screen_philosophy",
        title: "Screen Philosophy",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "site_specificity_theory",
        title: "Site Specificity Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "social_activism",
        title: "Social Activism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "soft_skills_theory",
        title: "Soft Skills Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "spinoza_studies",
        title: "Spinoza Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "structuralism",
        title: "Structuralism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "trauma_studies",
        title: "Trauma Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "urban_theory",
        title: "Urban Theory",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "visual_studies",
        title: "Visual Studies",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "vital_materialism",
        title: "Vital Materialism",
        category: "critical-theory",
    },
    QappCatalogEntry {
        id: "white_studies",
        title: "White Studies",
        category: "critical-theory",
    },
];

/// Unique category slugs present in the catalogue (sorted).
pub fn catalogue_categories() -> Vec<&'static str> {
    let mut cats: Vec<&str> = STUDIO_QAPP_CATALOG.iter().map(|e| e.category).collect();
    cats.sort_unstable();
    cats.dedup();
    cats
}

/// Human label for a category slug.
pub fn category_label(slug: &str) -> String {
    match slug {
        "social-sciences" => "Social Sciences".into(),
        "humanities" => "Humanities".into(),
        "natural-sciences" => "Natural Sciences".into(),
        "formal-sciences" => "Formal Sciences".into(),
        "area-studies" => "Area Studies".into(),
        "applied-liberal-arts" => "Applied Liberal Arts".into(),
        "emerging-interdisciplinary" => "Emerging Interdisciplinary".into(),
        "specialized-sciences" => "Specialized Sciences".into(),
        "pre-professional" => "Pre-Professional".into(),
        "language-regional" => "Language & Regional".into(),
        "arts-performance" => "Arts & Performance".into(),
        "advanced-subdisciplines" => "Advanced Sub-disciplines".into(),
        "niche-sciences" => "Niche Sciences".into(),
        "philosophy-theory" => "Philosophy & Theory".into(),
        "religious-studies" => "Religious Studies".into(),
        "literary-media" => "Literary & Media".into(),
        "linguistics-semiotics" => "Linguistics & Semiotics".into(),
        "intersectional-applied" => "Intersectional & Applied".into(),
        "historical-textual" => "Historical & Textual".into(),
        "critical-cultural" => "Critical & Cultural".into(),
        "interdisciplinary-stem" => "Interdisciplinary STEM".into(),
        "design-spatial" => "Design & Spatial".into(),
        "critical-theory" => "Critical Theory".into(),
        "platform" => "Platform".into(),
        "website" => "Websites".into(),
        other => other.replace('-', " "),
    }
}

fn primary_subject_for_uri(uri: &str) -> u64 {
    // FNV-1a 64 then mask to 60 bits — matches hypermedia join keys without full ingest.
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in uri.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

/// Build a Software-section library entry for one catalogue row.
pub fn catalog_entry_to_library(entry: QappCatalogEntry, now: u64) -> LibraryEntry {
    let uri = entry.asset_uri();
    let mut le = LibraryEntry {
        asset_uri: uri.clone(),
        primary_subject: primary_subject_for_uri(&uri),
        media_type: QAPP_MEDIA_TYPE.into(),
        quins: Vec::new(),
        topics: vec![
            "qapp".into(),
            "academic".into(),
            entry.category.into(),
            entry.id.replace('_', "-"),
        ],
        projects: vec![format!("category:{}", entry.category)],
        purposes: vec!["qapp".into(), "software".into(), "academic".into()],
        place: None,
        occurred_at: None,
        lat: None,
        lon: None,
        flags: Vec::new(),
        ingested_unix: now,
        excerpt: format!(
            "{} — studio QApp ({}). Early liberal-arts pane; catalogued for Software browse.",
            entry.title,
            category_label(entry.category)
        ),
        sensitivity: "public".into(),
        section: LibrarySection::Software.as_str().into(),
        commons_visibility: CommonsVisibility::None,
        cml_signals: Vec::new(),
        cml_concept_count: 0,
        cml_n3: String::new(),
        cof_html: String::new(),
        cof_segment_count: 0,
        cof_segment_index: 0,
        cof_profile: String::new(),
    };
    le.recompute_section();
    le
}

/// Result of seeding the studio QApp catalogue into the hypermedia store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QappSeedReport {
    pub total_catalog: usize,
    pub added: usize,
    pub updated: usize,
    pub categories: usize,
}

/// Idempotently seed all studio QApps into the Library **Software** section.
/// Existing `qapp://studio/*` entries are refreshed (category/topics) so re-seed stays consistent.
/// Non-qapp library entries are left untouched. Single load + single save (not N round-trips).
pub fn seed_studio_qapps_into_library(store: &HypermediaStore) -> std::io::Result<QappSeedReport> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut entries = store.load()?;
    let mut by_uri: std::collections::HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.asset_uri.clone(), i))
        .collect();
    let mut added = 0usize;
    let mut updated = 0usize;
    for row in STUDIO_QAPP_CATALOG {
        let uri = row.asset_uri();
        let mut entry = catalog_entry_to_library(*row, now);
        if let Some(&idx) = by_uri.get(&uri) {
            entry.ingested_unix = entries[idx].ingested_unix;
            entries[idx] = entry;
            updated += 1;
        } else {
            by_uri.insert(uri, entries.len());
            entries.push(entry);
            added += 1;
        }
    }
    store.replace_all(&entries)?;
    Ok(QappSeedReport {
        total_catalog: STUDIO_QAPP_CATALOG.len(),
        added,
        updated,
        categories: catalogue_categories().len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_nonempty_and_categorised() {
        assert!(STUDIO_QAPP_CATALOG.len() >= 200);
        assert!(catalogue_categories().len() >= 10);
        for e in STUDIO_QAPP_CATALOG {
            assert!(!e.id.is_empty());
            assert!(!e.category.is_empty());
            assert!(!e.asset_uri().is_empty());
        }
    }

    #[test]
    fn seed_is_idempotent_and_software_section() {
        let dir = tempfile::tempdir().unwrap();
        let store = HypermediaStore::open(dir.path()).unwrap();
        let r1 = seed_studio_qapps_into_library(&store).unwrap();
        assert_eq!(r1.added, STUDIO_QAPP_CATALOG.len());
        assert_eq!(r1.updated, 0);
        let soft = store.by_section(LibrarySection::Software).unwrap();
        assert_eq!(soft.len(), STUDIO_QAPP_CATALOG.len());
        assert!(soft.iter().all(|e| e.media_type == QAPP_MEDIA_TYPE));
        let r2 = seed_studio_qapps_into_library(&store).unwrap();
        assert_eq!(r2.added, 0);
        assert_eq!(r2.updated, STUDIO_QAPP_CATALOG.len());
        assert_eq!(
            store.by_section(LibrarySection::Software).unwrap().len(),
            STUDIO_QAPP_CATALOG.len()
        );
    }
}
