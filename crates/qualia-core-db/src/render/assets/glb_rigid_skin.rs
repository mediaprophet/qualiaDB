//! Exact rigid glTF skin → native bind-rest conversion. No invented bind poses.
use super::{matrix4_to_motor_and_scale, AssetError, GlbNodeRecord, Joint, Motor, SkinWeight};
use crate::render::skeletal::MAX_JOINTS;
use serde_json::Value;
use std::collections::HashMap;

pub(super) struct RigidSkin {
    pub joints: Vec<Joint>,
    pub joint_ids: Vec<u64>,
    pub node_to_joint: HashMap<usize, u16>,
    pub node_local: Vec<Motor>,
    pub channel_prefix: Vec<Motor>,
}
fn bad(message: &str) -> AssetError {
    AssetError::Parse(message.into())
}
pub(super) fn validate_hierarchy(gltf: &Value) -> Result<(), AssetError> {
    let nodes = gltf["nodes"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    if nodes.len() > u16::MAX as usize {
        return Err(bad("glTF node indices exceed native hierarchy range"));
    }
    let mut parent = vec![None; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if let Some(children) = node.get("children") {
            for child in children
                .as_array()
                .ok_or_else(|| bad("node children must be an array"))?
            {
                let child = child
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|n| *n < nodes.len())
                    .ok_or_else(|| bad("node child missing"))?;
                if parent[child].replace(index).is_some() {
                    return Err(bad("glTF node has multiple parents or duplicate children"));
                }
            }
        }
    }
    let mut queue = std::collections::VecDeque::new();
    for (i, p) in parent.iter().enumerate() {
        if p.is_none() {
            queue.push_back(i);
        }
    }
    let mut count = 0;
    while let Some(index) = queue.pop_front() {
        count += 1;
        if let Some(children) = nodes[index]["children"].as_array() {
            for child in children {
                queue.push_back(child.as_u64().unwrap() as usize);
            }
        }
    }
    if count != nodes.len() {
        return Err(bad("cyclic glTF node hierarchy"));
    }
    Ok(())
}
/// Guard legacy typed readers against sparse/external data and view overrun.
pub(super) fn checked_accessor(
    acc: &Value,
    views: &[Value],
    bin: &[u8],
    width: usize,
    limit: usize,
) -> Result<(), AssetError> {
    if acc.get("sparse").is_some() {
        return Err(bad(
            "sparse skin/animation accessors require sparse-preserving decoding",
        ));
    }
    let count = acc["count"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| bad("accessor count invalid"))?;
    if count == 0 || count > limit {
        return Err(bad("accessor exceeds native count budget"));
    }
    let index = acc["bufferView"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| bad("accessor buffer view missing"))?;
    let view = views
        .get(index)
        .ok_or_else(|| bad("accessor buffer view missing"))?;
    if view["buffer"].as_u64().unwrap_or(0) != 0 {
        return Err(bad("accessor requires admitted GLB BIN buffer"));
    }
    let base = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let length = view["byteLength"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| bad("accessor view length missing"))?;
    let relative = acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().unwrap_or(width as u64) as usize;
    let span = (count - 1)
        .checked_mul(stride)
        .and_then(|n| n.checked_add(width))
        .and_then(|n| n.checked_add(relative));
    if stride < width
        || span.is_none_or(|n| n > length)
        || base.checked_add(length).is_none_or(|n| n > bin.len())
    {
        return Err(bad("accessor range exceeds its buffer view"));
    }
    Ok(())
}
fn identity(motor: Motor) -> bool {
    let (rotation, translation) = motor.to_rotation_translation();
    (rotation[0].abs() - 1.0).abs() < 1e-5 && translation.iter().all(|v| v.abs() < 1e-5)
}
fn rigid_node(node: &GlbNodeRecord) -> Result<(), AssetError> {
    if node
        .scale
        .iter()
        .any(|v| !v.is_finite() || (*v - 1.0).abs() > 1e-5)
    {
        return Err(bad(
            "glTF scaled skin nodes require a scale-preserving native skeletal profile",
        ));
    }
    Ok(())
}

pub(super) fn parse_skins(
    gltf: &Value,
    nodes: &[GlbNodeRecord],
    accessors: &[Value],
    views: &[Value],
    bin: &[u8],
    weights: &mut [SkinWeight],
) -> Result<RigidSkin, AssetError> {
    let raw_skins = gltf["skins"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    if raw_skins.is_empty() {
        if !weights.is_empty() || nodes.iter().any(|n| n.skin_index.is_some()) {
            return Err(bad("glTF vertex skin weights lack a skin"));
        }
        return Ok(RigidSkin {
            joints: vec![],
            joint_ids: vec![],
            node_to_joint: HashMap::new(),
            node_local: vec![],
            channel_prefix: vec![],
        });
    }
    if raw_skins.len() != 1 {
        return Err(bad(
            "multiple glTF skins require separately bound native mesh/skin instances",
        ));
    }
    let skin = &raw_skins[0];
    let check_node = |index: usize| -> Result<(), AssetError> {
        rigid_node(nodes.get(index).ok_or_else(|| bad("skin node missing"))?)?;
        let raw = &gltf["nodes"][index];
        for (name, width) in [("translation", 3), ("rotation", 4), ("scale", 3)] {
            if let Some(values) = raw.get(name) {
                let values = values
                    .as_array()
                    .filter(|v| v.len() == width)
                    .ok_or_else(|| bad("skin node TRS shape invalid"))?;
                if values
                    .iter()
                    .any(|v| v.as_f64().is_none_or(|n| !(n as f32).is_finite()))
                {
                    return Err(bad("skin node TRS is non-finite"));
                }
                if name == "rotation"
                    && (values
                        .iter()
                        .map(|v| v.as_f64().unwrap().powi(2))
                        .sum::<f64>()
                        - 1.0)
                        .abs()
                        > 1e-5
                {
                    return Err(bad("skin node rotation is not unit length"));
                }
            }
        }
        if let Some(raw) = gltf["nodes"][index]["matrix"].as_array() {
            if raw.len() != 16 {
                return Err(bad("skin node matrix must contain 16 values"));
            }
            let mut matrix = [0.0; 16];
            for (slot, value) in matrix.iter_mut().zip(raw) {
                *slot = value
                    .as_f64()
                    .ok_or_else(|| bad("skin node matrix is not numeric"))?
                    as f32;
            }
            rigid_matrix(&matrix)?;
        }
        Ok(())
    };
    let raw = skin["joints"]
        .as_array()
        .ok_or_else(|| bad("skin.joints missing"))?;
    if raw.is_empty() || raw.len() > MAX_JOINTS {
        return Err(bad("glTF skin exceeds native joint budget"));
    }
    let mut source_nodes = Vec::with_capacity(raw.len());
    for index in raw {
        let index = index
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| bad("skin joint index invalid"))?;
        if index >= nodes.len() || source_nodes.contains(&index) {
            return Err(bad("skin joint node is missing or repeated"));
        }
        source_nodes.push(index);
    }
    // Current native packed mesh is in one rigid mesh frame; never erase an
    // authored mesh-node transform or silently conflate different skins.
    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.skin_index.is_some())
    {
        if node.skin_index != Some(0) || node.mesh_index.is_none() {
            return Err(bad("glTF skin binding has no matching mesh"));
        }
        let mut current = Some(index);
        for _ in 0..=nodes.len() {
            let Some(index) = current else { break };
            let n = &nodes[index];
            check_node(index)?;
            if !identity(n.local_motor) {
                return Err(bad("transformed glTF skinned mesh nodes require an instance-frame preserving native profile"));
            }
            current = n.parent.map(usize::from);
            if current.is_some_and(|p| p >= nodes.len()) {
                return Err(bad("skin mesh parent missing"));
            }
        }
        if current.is_some() {
            return Err(bad("cyclic glTF mesh ancestry"));
        }
    }
    let inverse_binds = match skin["inverseBindMatrices"].as_u64() {
        Some(index) => read_inverse_binds(
            accessors
                .get(index as usize)
                .ok_or_else(|| bad("inverse bind accessor missing"))?,
            views,
            bin,
        )?,
        None => vec![Motor::identity(); source_nodes.len()], // glTF specified identity default.
    };
    if inverse_binds.len() != source_nodes.len() {
        return Err(bad("inverse bind count differs from skin joint count"));
    }
    let mut source_parent = vec![None; source_nodes.len()];
    let mut defaults = vec![Motor::identity(); source_nodes.len()];
    for (old, &index) in source_nodes.iter().enumerate() {
        let mut current = Some(index);
        let mut pose = Motor::identity();
        let mut seen = Vec::new();
        while let Some(index) = current {
            if seen.contains(&index) {
                return Err(bad("cyclic glTF joint ancestry"));
            }
            seen.push(index);
            if index != source_nodes[old] {
                if let Some(parent) = source_nodes.iter().position(|&n| n == index) {
                    source_parent[old] = Some(parent);
                    break;
                }
            }
            let node = nodes
                .get(index)
                .ok_or_else(|| bad("joint ancestor missing"))?;
            check_node(index)?;
            pose = node.local_motor.mul(&pose).normalize();
            current = node.parent.map(usize::from);
        }
        defaults[old] = pose;
    }
    let mut order = Vec::with_capacity(source_nodes.len());
    while order.len() < source_nodes.len() {
        let before = order.len();
        for old in 0..source_nodes.len() {
            if !order.contains(&old) && source_parent[old].is_none_or(|p| order.contains(&p)) {
                order.push(old);
            }
        }
        if order.len() == before {
            return Err(bad("cyclic native joint ordering"));
        }
    }
    let mut remap = vec![0u16; order.len()];
    for (new, &old) in order.iter().enumerate() {
        remap[old] = new as u16;
    }
    for weight in weights {
        for lane in 0..4 {
            let old = weight.joints[lane] as usize;
            if old >= remap.len() {
                if weight.weights[lane] > 0.0 {
                    return Err(bad("vertex weight refers to absent skin joint"));
                }
                weight.joints[lane] = 0;
            } else {
                weight.joints[lane] = remap[old];
            }
        }
    }
    let mut result = RigidSkin {
        joints: vec![],
        joint_ids: vec![],
        node_to_joint: HashMap::new(),
        node_local: vec![],
        channel_prefix: vec![],
    };
    for &old in &order {
        let global_bind = inverse_binds[old].conjugate();
        let rest_local = source_parent[old]
            .map_or(global_bind, |parent| {
                inverse_binds[parent].mul(&global_bind)
            })
            .normalize();
        result.joints.push(Joint {
            parent: source_parent[old].map(|parent| remap[parent]),
            rest_local,
            inverse_bind: inverse_binds[old],
        });
        let name = nodes[source_nodes[old]].name.as_deref().unwrap_or("joint");
        result
            .joint_ids
            .push(crate::q_hash(&format!("urn:qualia:joint:{old}:{name}")));
        result.node_to_joint.insert(source_nodes[old], remap[old]);
        result.node_local.push(nodes[source_nodes[old]].local_motor);
        result.channel_prefix.push(
            defaults[old]
                .mul(&nodes[source_nodes[old]].local_motor.conjugate())
                .normalize(),
        );
    }
    Ok(result)
}

fn read_inverse_binds(
    accessor: &Value,
    views: &[Value],
    bin: &[u8],
) -> Result<Vec<Motor>, AssetError> {
    if accessor["componentType"].as_u64() != Some(5126) || accessor["type"].as_str() != Some("MAT4")
    {
        return Err(bad("inverse binds require FLOAT MAT4"));
    }
    let count = accessor["count"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| bad("inverse bind count invalid"))?;
    if count > MAX_JOINTS {
        return Err(bad("inverse binds exceed native joint budget"));
    }
    let view = views
        .get(
            accessor["bufferView"]
                .as_u64()
                .ok_or_else(|| bad("inverse bind view missing"))? as usize,
        )
        .ok_or_else(|| bad("inverse bind view missing"))?;
    if view["buffer"].as_u64().unwrap_or(0) != 0 {
        return Err(bad("inverse binds require the admitted GLB BIN buffer"));
    }
    let base = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let end = base
        .checked_add(
            view["byteLength"]
                .as_u64()
                .ok_or_else(|| bad("inverse bind view length missing"))? as usize,
        )
        .ok_or_else(|| bad("inverse bind range overflow"))?;
    let start = base
        .checked_add(accessor["byteOffset"].as_u64().unwrap_or(0) as usize)
        .ok_or_else(|| bad("inverse bind range overflow"))?;
    let stride = view["byteStride"].as_u64().unwrap_or(64) as usize;
    if stride < 64 {
        return Err(bad("inverse bind stride too small"));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let offset = i
            .checked_mul(stride)
            .and_then(|v| start.checked_add(v))
            .ok_or_else(|| bad("inverse bind range overflow"))?;
        let stop = offset
            .checked_add(64)
            .filter(|v| *v <= end)
            .ok_or_else(|| bad("inverse binds exceed their buffer view"))?;
        let bytes = bin
            .get(offset..stop)
            .ok_or_else(|| bad("inverse binds exceed BIN"))?;
        let matrix: [f32; 16] = std::array::from_fn(|c| {
            f32::from_le_bytes(bytes[c * 4..c * 4 + 4].try_into().unwrap())
        });
        out.push(rigid_matrix(&matrix)?);
    }
    Ok(out)
}

fn rigid_matrix(matrix: &[f32; 16]) -> Result<Motor, AssetError> {
    if matrix.iter().any(|v| !v.is_finite())
        || [matrix[3], matrix[7], matrix[11], matrix[15] - 1.0]
            .iter()
            .any(|v| v.abs() > 1e-5)
    {
        return Err(bad("inverse bind is not a finite affine matrix"));
    }
    let dot = |a: usize, b: usize| {
        (0..3)
            .map(|r| matrix[a * 4 + r] * matrix[b * 4 + r])
            .sum::<f32>()
    };
    let det = matrix[0] * (matrix[5] * matrix[10] - matrix[9] * matrix[6])
        - matrix[4] * (matrix[1] * matrix[10] - matrix[9] * matrix[2])
        + matrix[8] * (matrix[1] * matrix[6] - matrix[5] * matrix[2]);
    if (0..3).any(|a| (dot(a, a) - 1.0).abs() > 1e-5)
        || dot(0, 1).abs() > 1e-5
        || dot(0, 2).abs() > 1e-5
        || dot(1, 2).abs() > 1e-5
        || (det - 1.0).abs() > 1e-5
    {
        return Err(bad(
            "scaled/sheared/reflected inverse binds require a nonrigid native profile",
        ));
    }
    Ok(matrix4_to_motor_and_scale(matrix).0)
}
