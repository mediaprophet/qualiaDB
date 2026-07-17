pub mod wellfair;
pub mod browser_panes;
pub mod camera_controls;
pub mod chat_graph;
pub mod contextual_workspace;
pub mod dashboard;
pub mod diffusion_visualizer;
pub mod hardware_configurator;
pub mod health_monitor;
pub mod honesty_chip;
pub mod command_palette;
pub mod virtualized_list;
pub mod llm_harness;
pub mod nexus;
pub mod ontology_import_wizard;
pub mod personal_ontology;
pub mod selection_sidebar;
pub mod qapp_dispatcher;
pub mod qapps;
pub mod render_preview;
pub mod scene_interaction;
pub mod shoelace;
pub mod ten_d_browser;
pub mod native_gpu_viewport;

// 10D Integration Components
// Fixed: Renamed .rsx files to .rs for Rust module system compatibility
pub mod anatomy_test;
pub mod epistemic_status;
pub mod hardware_capabilities;
pub mod temporal_scrubber;

pub mod about_page;
pub mod agent_config;
pub mod agreements_rights;
pub mod benchmark_harness;
pub mod bioinformatics_lab;
pub mod browser_unavailable;
pub mod chemistry_modeler;
pub mod connect_chat;
pub mod tool_use_card;
pub mod conduct_banner;
pub mod social_hub;
pub mod directory_pane;
pub mod domains_pane;
pub mod cli_bridge;
pub mod clinical_risk_scorer;
pub mod comorbidity_analyzer;
pub mod connect_pane;
pub mod deontic_logic_editor;
pub mod dicom_viewer;
pub mod ebpf_filter_manager;
pub mod extension_bus;
pub mod gbm_simulator;
pub mod health_vital_monitor;
pub mod inference_monitor;
pub mod key_vault_manager;
pub mod lora_manager;
pub mod matrix_lab;
pub mod mcp_inspector;
pub mod model_lifecycle;
pub mod n3_logic_studio;
pub mod logic_modeler;
pub mod ode_solver;
pub mod p2p_dashboard;
pub mod physics_simulator;
pub mod portfolio_analyzer;
pub mod problems_pane;
pub mod provenance_graph;
pub mod q42_volume_manager;
pub mod qaoa_explorer;
pub mod qpu_optimizer;
pub mod qpu_providers;
pub mod quantum_dft;
pub mod rdf_star_editor;
pub mod risk_engine;
pub mod settings_page;
pub mod shacl_validator;
pub mod solid_ldp_browser;
pub mod sparql_explorer;
pub mod statistical_analysis;
pub mod storage_driver_config;
pub mod wal_inspector;
pub mod vision_workbench;
pub mod listen_workbench;
pub mod webtorrent_seeder;
pub mod zk_proof_studio;

// Social Sciences QApps
pub mod african_american_studies_qapp;
pub mod anthropology_qapp;
pub mod archaeology_qapp;
pub mod area_and_regional_studies_qapp;
pub mod cultural_studies_qapp;
pub mod economics_qapp;
pub mod gender_and_sexuality_studies_qapp;
pub mod geography_human_geography_qapp;
pub mod history_qapp;
pub mod international_relations_qapp;
pub mod political_science_qapp;
pub mod psychology_qapp;
pub mod sociology_qapp;

// Humanities QApps
pub mod art_history_qapp;
pub mod classics_qapp;
pub mod comparative_literature_qapp;
pub mod creative_writing_qapp;
pub mod dance_qapp;
pub mod english_language_and_literature_qapp;
pub mod ethics_qapp;
pub mod film_and_media_studies_qapp;
pub mod foreign_languages_and_literatures_qapp;
pub mod linguistics_qapp;
pub mod music_history_qapp;
pub mod music_performance_qapp;
pub mod music_theory_qapp;
pub mod philosophy_qapp;
pub mod religion_and_theology_qapp;
pub mod studio_art_qapp;
pub mod theater_and_drama_qapp;

// Natural Sciences QApps
pub mod astronomy_qapp;
pub mod astrophysics_qapp;
pub mod biology_qapp;
pub mod botany_qapp;
pub mod chemistry_qapp;
pub mod earth_science_qapp;
pub mod ecology_qapp;
pub mod environmental_science_qapp;
pub mod evolutionary_biology_qapp;
pub mod geology_qapp;
pub mod neuroscience_qapp;
pub mod oceanography_qapp;
pub mod physics_qapp;
pub mod zoology_qapp;

// Formal Sciences QApps
pub mod computer_science_qapp;
pub mod logic_qapp;
pub mod mathematics_qapp;
pub mod statistics_qapp;

// Interdisciplinary and Area Studies QApps
pub mod american_studies_qapp;
pub mod asian_studies_qapp;
pub mod cognitive_science_qapp;
pub mod communication_studies_qapp;
pub mod european_studies_qapp;
pub mod folklore_and_mythology_qapp;
pub mod global_studies_qapp;
pub mod history_of_science_and_medicine_qapp;
pub mod indigenous_and_native_american_studies_qapp;
pub mod jewish_studies_qapp;
pub mod latin_american_studies_qapp;
pub mod medieval_and_renaissance_studies_qapp;
pub mod middle_eastern_studies_qapp;
pub mod peace_and_conflict_studies_qapp;
pub mod rhetoric_and_composition_qapp;
pub mod science_technology_and_society_sts_qapp;
pub mod urban_studies_qapp;

// Applied and Specialized Liberal Arts QApps
pub mod criminology_and_criminal_justice_qapp;
pub mod education_studies_qapp;
pub mod journalism_qapp;
pub mod library_and_information_science_qapp;
pub mod museum_studies_qapp;
pub mod public_health_qapp;
pub mod public_policy_qapp;

// Emerging and Specialized Interdisciplinary Fields QApps
pub mod african_studies_qapp;
pub mod celtic_studies_qapp;
pub mod chicano_and_latino_studies_qapp;
pub mod digital_humanities_qapp;
pub mod disability_studies_qapp;
pub mod environmental_humanities_qapp;
pub mod ethnomusicology_qapp;
pub mod food_studies_qapp;
pub mod game_studies_qapp;
pub mod human_rights_studies_qapp;
pub mod medical_humanities_qapp;
pub mod oceanic_and_pacific_island_studies_qapp;
pub mod postcolonial_studies_qapp;
pub mod poverty_and_inequality_studies_qapp;
pub mod queer_studies_qapp;
pub mod scandinavian_studies_qapp;
pub mod slavic_studies_qapp;
pub mod sound_studies_qapp;
pub mod sustainability_studies_qapp;
pub mod translation_studies_qapp;
pub mod visual_and_critical_studies_qapp;

// Specialized Natural and Social Sciences QApps
pub mod astrobiology_qapp;
pub mod behavioral_economics_qapp;
pub mod biomathematics_qapp;
pub mod biophysics_qapp;
pub mod demography_and_population_studies_qapp;
pub mod geophysics_qapp;
pub mod planetary_science_qapp;
pub mod political_economy_qapp;
pub mod social_psychology_qapp;

// Pre-Professional QApps
pub mod art_conservation_qapp;
pub mod arts_management_and_administration_qapp;
pub mod bioethics_qapp;
pub mod curatorial_studies_qapp;
pub mod leadership_studies_qapp;
pub mod legal_studies_qapp;
pub mod social_work_qapp;

// Highly Specialized Language and Regional Studies QApps
pub mod ancient_near_eastern_studies_qapp;
pub mod appalachian_studies_qapp;
pub mod arctic_studies_qapp;
pub mod balkan_studies_qapp;
pub mod caribbean_studies_qapp;
pub mod central_asian_studies_qapp;
pub mod egyptology_qapp;
pub mod francophone_studies_qapp;
pub mod germanic_languages_and_literatures_qapp;
pub mod hispanic_and_luso_brazilian_studies_qapp;
pub mod philology_qapp;
pub mod romance_languages_and_literatures_qapp;
pub mod south_asian_studies_qapp;
pub mod southeast_asian_studies_qapp;

// Specialized Arts and Performance QApps
pub mod animation_and_digital_arts_qapp;
pub mod book_arts_and_papermaking_qapp;
pub mod ceramics_qapp;
pub mod cinematography_qapp;
pub mod dramaturgy_qapp;
pub mod musicology_qapp;
pub mod photography_qapp;
pub mod playwriting_qapp;
pub mod printmaking_qapp;
pub mod screenwriting_qapp;
pub mod sculpture_qapp;

// Advanced Sub-Disciplines QApps
pub mod applied_linguistics_qapp;
pub mod family_studies_qapp;
pub mod gerontology_qapp;
pub mod penology_qapp;
pub mod psycholinguistics_qapp;
pub mod social_and_cultural_analysis_qapp;
pub mod sociolinguistics_qapp;

// Niche Natural and Physical Sciences QApps
pub mod atmospheric_science_qapp;
pub mod behavioral_ecology_qapp;
pub mod kinesiology_and_movement_studies_qapp;
pub mod marine_biology_qapp;
pub mod materials_science_qapp;
pub mod meteorology_qapp;
pub mod mycology_qapp;
pub mod paleontology_qapp;
pub mod sports_studies_qapp;

// Advanced Philosophical and Theoretical Studies QApps
pub mod aesthetics_qapp;
pub mod epistemology_qapp;
pub mod metaphysics_qapp;
pub mod phenomenology_qapp;
pub mod philosophy_of_mind_qapp;
pub mod philosophy_of_religion_qapp;
pub mod philosophy_of_science_qapp;
pub mod social_and_political_philosophy_qapp;

// Specialized Religious and Theological Studies QApps
pub mod biblical_studies_qapp;
pub mod buddhist_studies_qapp;
pub mod canon_law_qapp;
pub mod hindu_studies_qapp;
pub mod islamic_studies_qapp;
pub mod missiology_qapp;
pub mod patristics_qapp;

// Advanced Literary and Media Studies QApps
pub mod arthurian_studies_qapp;
pub mod comics_and_graphic_novel_studies_qapp;
pub mod fan_studies_qapp;
pub mod poetry_and_poetics_qapp;
pub mod science_fiction_and_fantasy_studies_qapp;
pub mod utopian_studies_qapp;

// Deep Linguistics and Semiotics QApps
pub mod historical_linguistics_qapp;
pub mod morphology_qapp;
pub mod pragmatics_qapp;
pub mod semantics_qapp;
pub mod semiotics_qapp;
pub mod syntax_qapp;

// Specialized Intersectional and Applied Studies QApps
pub mod disaster_studies_qapp;
pub mod futures_studies_and_foresight_qapp;
pub mod leisure_studies_qapp;
pub mod philanthropy_and_nonprofit_studies_qapp;

// Advanced Historical and Textual Studies QApps
pub mod architectural_history_qapp;
pub mod childrens_literature_qapp;
pub mod history_of_art_and_architecture_qapp;
pub mod intellectual_history_qapp;
pub mod maritime_history_qapp;
pub mod military_history_qapp;
pub mod oral_history_qapp;
pub mod paleography_qapp;
pub mod public_history_qapp;
pub mod textual_criticism_qapp;

// Critical and Cultural Sub-Disciplines QApps
pub mod animal_studies_human_animal_studies_qapp;
pub mod body_studies_qapp;
pub mod critical_race_and_ethnic_studies_qapp;
pub mod critical_theory_qapp;
pub mod deaf_studies_qapp;
pub mod diaspora_studies_qapp;
pub mod fat_studies_qapp;
pub mod indigenous_language_revitalization_qapp;
pub mod material_culture_studies_qapp;
pub mod memory_studies_qapp;
pub mod whiteness_studies_qapp;

// Interdisciplinary Science and Mathematics QApps
pub mod biogeochemistry_qapp;
pub mod bioinformatics_qapp;
pub mod chemical_physics_qapp;
pub mod computational_linguistics_qapp;
pub mod cryptography_qapp;
pub mod environmental_chemistry_qapp;
pub mod geochemistry_qapp;
pub mod mathematical_biology_qapp;
pub mod mathematical_economics_qapp;
pub mod systems_biology_qapp;

// Design, Media, and Spatial Studies QApps
pub mod architectural_studies_qapp;
pub mod cyberculture_studies_qapp;
pub mod environmental_design_qapp;
pub mod landscape_studies_qapp;
pub mod media_ecology_qapp;
pub mod spatial_data_science_qapp;
pub mod urban_ecology_qapp;
pub mod urban_planning_and_design_qapp;

// Advanced Critical Theory and Cultural Critique QApps
pub mod affect_theory_qapp;
pub mod biopolitics_qapp;
pub mod capital_studies_qapp;
pub mod critical_disability_studies_qapp;
pub mod critical_film_studies_qapp;
pub mod critical_gentrification_studies_qapp;
pub mod cultural_ecology_qapp;
pub mod decolonial_studies_qapp;
pub mod eco_critical_theory_qapp;
pub mod eco_feminism_qapp;
pub mod eco_queer_theory_qapp;
pub mod environmental_justice_qapp;
pub mod film_philosophy_qapp;
pub mod gender_studies_qapp;
pub mod global_critical_studies_qapp;
pub mod grassroots_studies_qapp;
pub mod grief_studies_qapp;
pub mod haunted_humanities_qapp;
pub mod hermeneutics_qapp;
pub mod historiography_qapp;
pub mod ideology_critique_qapp;
pub mod indigenous_feminisms_qapp;
pub mod integral_studies_qapp;
pub mod intermedia_studies_qapp;
pub mod landscape_phenomenology_qapp;
pub mod liberation_studies_qapp;
pub mod literature_and_law_qapp;
pub mod materialist_aesthetics_qapp;
pub mod media_theory_qapp;
pub mod metamodernism_qapp;
pub mod post_critical_pedagogy_qapp;
pub mod posthumanities_qapp;
pub mod poststructuralism_qapp;
pub mod psychoanalysis_qapp;
pub mod queer_cinema_studies_qapp;
pub mod queer_theory_qapp;
pub mod race_critical_theory_qapp;
pub mod race_studies_qapp;
pub mod race_theory_qapp;
pub mod radical_media_studies_qapp;
pub mod regionalism_qapp;
pub mod revisionist_critical_theory_qapp;
pub mod rural_studies_qapp;
pub mod screen_philosophy_qapp;
pub mod site_specificity_theory_qapp;
pub mod social_activism_qapp;
pub mod soft_skills_theory_qapp;
pub mod spinoza_studies_qapp;
pub mod structuralism_qapp;
pub mod trauma_studies_qapp;
pub mod urban_theory_qapp;
pub mod visual_studies_qapp;
pub mod vital_materialism_qapp;
pub mod white_studies_qapp;

pub mod qapp_engine;
pub use qapp_engine::*;
