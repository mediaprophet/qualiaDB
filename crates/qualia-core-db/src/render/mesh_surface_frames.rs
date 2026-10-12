//! Geometry-aligned UV charts with explicit seam vertices and preserved normals.
//! Cold construction over caller-owned buffers; no private renderer topology.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceFrameError { InvalidInput, OutputTooSmall, BudgetExceeded }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceFrameReceipt { pub vertex_count: usize, pub triangle_count: usize }

fn valid(positions: &[[f32;3]], triangles: &[[u32;3]]) -> bool {
    !positions.is_empty() && !triangles.is_empty()
        && positions.iter().flatten().all(|v|v.is_finite() && v.abs()<1e10)
        && triangles.iter().flatten().all(|i|(*i as usize)<positions.len())
}
fn face(positions: &[[f32;3]], triangle: [u32;3]) -> [f32;3] {
    let [a,b,c]=triangle.map(|i|positions[i as usize]);
    let u=[b[0]-a[0],b[1]-a[1],b[2]-a[2]];
    let v=[c[0]-a[0],c[1]-a[1],c[2]-a[2]];
    [u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
}
fn unit(v: [f32;3]) -> [f32;3] {
    let length=(v[0]*v[0]+v[1]*v[1]+v[2]*v[2]).sqrt();
    if !length.is_finite() || length<1e-12 {[0.0,1.0,0.0]} else {v.map(|x|x/length)}
}

/// Area-weighted source vertex normals. Shared vertices remain smooth, and
/// coincident but independently authored vertices retain their own hard edges.
pub fn vertex_normals_into(positions: &[[f32;3]], triangles: &[[u32;3]], out: &mut [[f32;3]])
    -> Result<usize,SurfaceFrameError> {
    if !valid(positions,triangles) {return Err(SurfaceFrameError::InvalidInput);}
    if out.len()<positions.len() {return Err(SurfaceFrameError::OutputTooSmall);}
    Ok(write_normals(positions,triangles.iter().copied(),out))
}

/// Equivalent index-stream entry for scene merges and geometry fit updates.
pub fn vertex_normals_from_indices_into(positions: &[[f32;3]], indices: &[u32], out: &mut [[f32;3]])
    -> Result<usize,SurfaceFrameError> {
    if positions.is_empty() || indices.is_empty() || indices.len()%3!=0
        || positions.iter().flatten().any(|v|!v.is_finite() || v.abs()>=1e10)
        || indices.iter().any(|i|*i as usize>=positions.len())
    {return Err(SurfaceFrameError::InvalidInput);}
    if out.len()<positions.len() {return Err(SurfaceFrameError::OutputTooSmall);}
    Ok(write_normals(positions,indices.chunks_exact(3).map(|t|[t[0],t[1],t[2]]),out))
}

fn write_normals(positions: &[[f32;3]], triangles: impl Iterator<Item=[u32;3]>, out: &mut [[f32;3]]) -> usize {
    out[..positions.len()].fill([0.0;3]);
    for triangle in triangles {
        let n=face(positions,triangle);
        for index in triangle {for k in 0..3 {out[index as usize][k]+=n[k];}}
    }
    for normal in &mut out[..positions.len()] {*normal=unit(*normal);}
    positions.len()
}

/// Triangle charts choose the dominant face axis and duplicate corners at UV
/// seams. Supplied source normals survive duplication, keeping curved coachwork
/// smooth; flat shading may be explicitly requested for metal sheet/box assets.
/// Triangle order is unchanged, so existing submesh/material ranges still apply.
pub fn box_project_mesh_into(positions: &[[f32;3]], triangles: &[[u32;3]],
    source_normals: &[[f32;3]], metres_per_repeat: f32, smooth: bool,
    out_positions: &mut [[f32;3]], out_triangles: &mut [[u32;3]],
    out_normals: &mut [[f32;3]], out_uv: &mut [[f32;2]],
    chart_vertices: &mut [[u32;6]],
) -> Result<SurfaceFrameReceipt,SurfaceFrameError> {
    box_project_mesh_with_sources_into(positions,triangles,source_normals,metres_per_repeat,smooth,
        out_positions,out_triangles,out_normals,out_uv,chart_vertices,None)
}

/// Also emits the original vertex index for each chart vertex. Skin influences
/// and other vertex-aligned fields must follow this lineage through UV seams.
pub fn box_project_mesh_with_sources_into(positions: &[[f32;3]], triangles: &[[u32;3]],
    source_normals: &[[f32;3]], metres_per_repeat: f32, smooth: bool,
    out_positions: &mut [[f32;3]], out_triangles: &mut [[u32;3]],
    out_normals: &mut [[f32;3]], out_uv: &mut [[f32;2]],
    chart_vertices: &mut [[u32;6]], mut source_vertices: Option<&mut [u32]>,
) -> Result<SurfaceFrameReceipt,SurfaceFrameError> {
    if !valid(positions,triangles) || source_normals.len()!=positions.len()
        || source_normals.iter().flatten().any(|v|!v.is_finite())
        || !metres_per_repeat.is_finite() || metres_per_repeat<=0.0
    {return Err(SurfaceFrameError::InvalidInput);}
    let count=triangles.len().checked_mul(3).ok_or(SurfaceFrameError::BudgetExceeded)?;
    // Combined new public streams remain below 10 MiB per authored mesh.
    if count>262_144 {return Err(SurfaceFrameError::BudgetExceeded);}
    if out_positions.len()<count || out_normals.len()<count || out_uv.len()<count
        || out_triangles.len()<triangles.len() || chart_vertices.len()<positions.len()
        || source_vertices.as_ref().is_some_and(|out|out.len()<count)
    {return Err(SurfaceFrameError::OutputTooSmall);}
    chart_vertices[..positions.len()].fill([u32::MAX;6]);
    let mut vertices=0usize;
    for (i,triangle) in triangles.iter().enumerate() {
        let normal=unit(face(positions,*triangle));
        let axis=if normal[0].abs()>=normal[1].abs() && normal[0].abs()>=normal[2].abs() {0}
            else if normal[1].abs()>=normal[2].abs() {1} else {2};
        let (u,v)=match axis {0=>(2,1),1=>(0,2),_=>(0,1)};
        let sign=if normal[axis]<0.0 {-1.0} else {1.0};
        let chart=axis*2+usize::from(sign<0.0);
        for (corner,index) in triangle.iter().enumerate() {
            let source=*index as usize;
            if smooth && chart_vertices[source][chart]!=u32::MAX {
                out_triangles[i][corner]=chart_vertices[source][chart];continue;
            }
            let target=vertices; vertices+=1; let p=positions[source];
            out_positions[target]=p;
            if let Some(out)=source_vertices.as_deref_mut() {out[target]=*index;}
            out_normals[target]=if smooth {unit(source_normals[source])} else {normal};
            out_uv[target]=[p[u]*sign/metres_per_repeat,p[v]/metres_per_repeat];
            out_triangles[i][corner]=target as u32;
            if smooth {chart_vertices[source][chart]=target as u32;}
        }
    }
    Ok(SurfaceFrameReceipt {vertex_count:vertices,triangle_count:triangles.len()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertical_side_face_has_noncollapsed_uvs_and_keeps_custom_normal() {
        let p=[[2.0,0.0,0.0],[2.0,1.0,0.0],[2.0,0.0,1.0]];
        let mut positions=[[0.0;3];3]; let mut triangles=[[0;3];1];
        let mut normals=[[0.0;3];3]; let mut uv=[[0.0;2];3];
        box_project_mesh_into(&p,&[[0,1,2]],&[[0.8,0.6,0.0];3],1.0,true,
            &mut positions,&mut triangles,&mut normals,&mut uv,&mut [[u32::MAX;6];3]).unwrap();
        assert_eq!(uv,[[0.0,0.0],[0.0,1.0],[1.0,0.0]]);
        assert_eq!(normals,[[0.8,0.6,0.0];3]); assert_eq!(positions,p);
    }
    #[test]
    fn undersized_output_preserves_all_streams() {
        let p=[[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]];
        let mut positions=[[9.0;3];3]; let mut triangles=[[8;3];1];
        let mut normals=[[7.0;3];3]; let mut uv=[[6.0;2];2];
        assert_eq!(box_project_mesh_into(&p,&[[0,1,2]],&[[0.0,0.0,1.0];3],1.0,false,
            &mut positions,&mut triangles,&mut normals,&mut uv,&mut [[u32::MAX;6];3]),Err(SurfaceFrameError::OutputTooSmall));
        assert_eq!(positions,[[9.0;3];3]); assert_eq!(triangles,[[8;3];1]);
        assert_eq!(normals,[[7.0;3];3]); assert_eq!(uv,[[6.0;2];2]);
    }
    #[test]
    fn one_smooth_chart_reuses_vertices_without_changing_triangle_order() {
        let p=[[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0],[1.0,1.0,0.0]];
        let mut positions=[[0.0;3];6];let mut triangles=[[0;3];2];
        let mut normals=[[0.0;3];6];let mut uv=[[0.0;2];6];let mut charts=[[u32::MAX;6];4];
        let receipt=box_project_mesh_into(&p,&[[0,1,2],[1,3,2]],&[[0.0,0.0,1.0];4],1.0,true,
            &mut positions,&mut triangles,&mut normals,&mut uv,&mut charts).unwrap();
        assert_eq!(receipt.vertex_count,4);assert_eq!(receipt.triangle_count,2);
        assert_eq!(triangles,[[0,1,2],[1,3,2]]);
    }
    #[test]
    fn seam_lineage_keeps_vertex_fields_attached_to_source() {
        let p=[[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]];
        let mut positions=[[0.0;3];6];let mut triangles=[[0;3];2];
        let mut normals=[[0.0;3];6];let mut uv=[[0.0;2];6];
        let mut charts=[[u32::MAX;6];4];let mut sources=[u32::MAX;6];
        let receipt=box_project_mesh_with_sources_into(&p,&[[0,1,2],[0,3,1]],&[[0.0,1.0,0.0];4],
            1.0,true,&mut positions,&mut triangles,&mut normals,&mut uv,&mut charts,Some(&mut sources)).unwrap();
        assert_eq!(receipt.vertex_count,6);
        assert_eq!(sources,[0,1,2,0,3,1]);
        for i in 0..receipt.vertex_count {assert_eq!(positions[i],p[sources[i] as usize]);}
    }
}
