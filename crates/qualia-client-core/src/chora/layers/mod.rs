pub mod catalog;
pub mod mesh_gen;
pub mod nasa_gibs;
pub mod starfield;

pub use catalog::{
    all_categories, find_layer, layers_by_category, LayerCategory, LayerDefinition, LayerId,
    LayerSource, LAYER_CATALOG,
};
pub use mesh_gen::{generate_sphere_mesh, generate_starfield_mesh, generate_terrain_mesh};
