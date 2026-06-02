use qualia_core_db::git_bridge;

fn main() {
    println!("--- Qualia-DB Git Exporter CLI ---");
    let project_id = "test_cooperative_project_001";
    
    println!("Generating Git Fast-Export stream for Project: {}", project_id);
    let stream = git_bridge::generate_fast_export_stream(project_id);
    
    println!("\n[EXPORT STREAM START]\n");
    println!("{}", stream);
    println!("[EXPORT STREAM END]\n");
    
    println!("Success. You can pipe this output directly into 'git fast-import'.");
}
