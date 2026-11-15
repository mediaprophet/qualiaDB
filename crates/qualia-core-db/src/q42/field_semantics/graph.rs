use super::*;
/// Kahn traversal with caller workspace. Worst-case node scans are O(V^2),
/// lookups O(log V) per edge, and metadata lookup O(V) per outgoing scan.
/// Every rescan/edge is charged to the shared work budget, not just edge count.
fn acyclic(
    ctx: &Context<'_, '_>,
    nodes: &mut [NodeState],
    limits: Limits,
    mut outgoing: impl FnMut(usize, &mut dyn FnMut(u32) -> Result<(), Error>) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut edges = 0usize;
    for index in 0..nodes.len() {
        ctx.charge_work(1)?;
        outgoing(index, &mut |target| {
            ctx.charge_work(64)?; // conservative binary-search bound for u32 IDs
            edges = edges.checked_add(1).ok_or(Error::Overflow)?;
            if edges > limits.edges {
                return Err(Error::Limit);
            }
            let i = nodes
                .binary_search_by_key(&target, |n| n.id)
                .map_err(|_| Error::Reference)?;
            nodes[i].indegree = nodes[i].indegree.checked_add(1).ok_or(Error::Overflow)?;
            Ok(())
        })?;
    }
    for _ in 0..nodes.len() {
        let mut chosen = None;
        for (index, node) in nodes.iter().enumerate() {
            ctx.charge_work(1)?;
            if !node.done && node.indegree == 0 {
                chosen = Some(index);
                break;
            }
        }
        let index = chosen.ok_or(Error::Cycle)?;
        let depth = nodes[index].depth;
        nodes[index].done = true;
        outgoing(index, &mut |target| {
            ctx.charge_work(64)?;
            let i = nodes
                .binary_search_by_key(&target, |n| n.id)
                .map_err(|_| Error::Reference)?;
            let next = depth.checked_add(1).ok_or(Error::Overflow)?;
            if next > limits.depth {
                return Err(Error::Depth);
            }
            nodes[i].depth = nodes[i].depth.max(next);
            nodes[i].indegree = nodes[i].indegree.checked_sub(1).ok_or(Error::Reference)?;
            Ok(())
        })?;
    }
    Ok(())
}
pub(super) fn metadata(
    ctx: &Context<'_, '_>,
    nodes: &mut [NodeState],
    limits: Limits,
) -> Result<(), Error> {
    for (node, block) in nodes.iter_mut().zip(ctx.view.blocks()) {
        ctx.charge_work(1)?;
        *node = NodeState {
            id: block.id(),
            ..NodeState::default()
        };
    }
    acyclic(ctx, nodes, limits, |index, visit| {
        ctx.charge_work(index as u64 + 1)?;
        let block = ctx.view.blocks().nth(index).ok_or(Error::Reference)?;
        for field in block.fields() {
            ctx.charge_work(1)?;
            if field.type_tag() == 8 {
                for target in field.value().chunks_exact(4) {
                    visit(u32_at(target, 0))?;
                }
            } else if field.type_tag() == 9 {
                for target in field.value().chunks_exact(4) {
                    ctx.entry(u32_at(target, 0))?;
                }
            }
        }
        Ok(())
    })
}
pub(super) fn ancestry(
    ctx: &Context<'_, '_>,
    nodes: &mut [NodeState],
    limits: Limits,
) -> Result<(), Error> {
    for (index, node) in nodes.iter_mut().enumerate() {
        ctx.charge_work(1)?;
        *node = NodeState {
            id: index as u32 + 1,
            ..NodeState::default()
        };
    }
    acyclic(ctx, nodes, limits, |index, visit| {
        let entry = ctx.entry(index as u32 + 1)?;
        let descriptor = ctx.require_schema(entry.descriptor_id(), 3)?;
        ctx.charge_work(descriptor.raw_bytes().len() as u64)?;
        if let Some(sources) = field(descriptor, 17) {
            for target in sources.value().chunks_exact(4) {
                visit(u32_at(target, 0))?;
            }
        }
        Ok(())
    })
}
