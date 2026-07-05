import os

def main():
    base_dir = r"c:\Projects\qualia-27062026\crates\qualia-core-db\src"
    
    stat_file = os.path.join(base_dir, r"specialized_libs\computational_geometry\statistical_manifold.rs")
    with open(stat_file, "r", encoding="utf-8") as f:
        content = f.read()
    
    content = content.replace("let _q = ", "let q = ")
    content = content.replace("let q = [0.3f32, 0.7];", "let _q = [0.3f32, 0.7];")
    
    with open(stat_file, "w", encoding="utf-8") as f:
        f.write(content)
        
    query_file = os.path.join(base_dir, r"specialized_libs\computational_geometry\query_frontend.rs")
    with open(query_file, "r", encoding="utf-8") as f:
        content = f.read()
    
    content = content.replace(
        "build_bvh_recursive, build_kd_tree_3d, Aabb, Point3,",
        "build_bvh_recursive, build_kd_tree_3d, Aabb, Point3, BVH_NODE_SIZE, KD_NODE_SIZE,\n        MAX_BVH_DEPTH, MAX_KD_DEPTH,"
    )
    
    with open(query_file, "w", encoding="utf-8") as f:
        f.write(content)

    print("Fixed.")

if __name__ == "__main__":
    main()
