import os
import re

def main():
    base_dir = r"c:\Projects\qualia-27062026\crates\qualia-core-db\src"
    
    file_1 = os.path.join(base_dir, r"container_10d\conformance.rs")
    with open(file_1, "r", encoding="utf-8") as f:
        c1 = f.read()
    c1 = c1.replace(
        "use super::SectionDescriptor;",
        "use crate::container_10d::crc32c::crc32c;\n    use super::SectionDescriptor;"
    )
    with open(file_1, "w", encoding="utf-8") as f:
        f.write(c1)
        
    file_2 = os.path.join(base_dir, r"container_10d\node_section.rs")
    with open(file_2, "r", encoding="utf-8") as f:
        c2 = f.read()
    c2 = c2.replace(
        "use super::AXIS_XYZ;",
        "use crate::container_10d::axis_role::AXIS_ORDER;\n    use super::AXIS_XYZ;"
    )
    with open(file_2, "w", encoding="utf-8") as f:
        f.write(c2)
        
    file_3 = os.path.join(base_dir, r"specialized_libs\computational_geometry\delaunay_2.rs")
    with open(file_3, "r", encoding="utf-8") as f:
        c3 = f.read()
    c3 = c3.replace(
        "use super::Point2;",
        "use super::hull::convex_hull_indices_2;\n    use super::Point2;"
    )
    with open(file_3, "w", encoding="utf-8") as f:
        f.write(c3)

    print("Fixed.")

if __name__ == "__main__":
    main()
