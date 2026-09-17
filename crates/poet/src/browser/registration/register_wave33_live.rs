//! Wave 33 leftover Live chains — Animation / numeric Ode / HbbTV.

use super::*;

fn live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
    icon: &'static str,
    ontology_prefix: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: ontology_prefix.into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn anim_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool(
            "animation:live_spring_step",
            "Spring step",
            "Animation.spring_step",
            "Step a spring via Animation.spring_step.",
            "3d",
            "hm",
        ),
        live_tool(
            "animation:live_sclerp_step",
            "ScLERP step",
            "Animation.sclerp_step",
            "Screw-linear interpolate motors via Animation.sclerp_step.",
            "3d",
            "hm",
        ),
        live_tool(
            "animation:live_squad_step",
            "SQUAD step",
            "Animation.squad_step",
            "Squad-interpolate quaternions via Animation.squad_step.",
            "3d",
            "hm",
        ),
        live_tool(
            "animation:live_list_presets",
            "List presets",
            "Animation.list_presets",
            "List animation presets via Animation.list_presets.",
            "3d",
            "hm",
        ),
    ]
}

pub(super) fn ode_num_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool(
            "scientific:num_ode_rk4",
            "RK4 integrate",
            "Ode.rk4_integrate",
            "Multi-step RK4 via Ode.rk4_integrate.",
            "lab",
            "sci",
        ),
        live_tool(
            "scientific:num_ode_dopri5",
            "DOPRI5",
            "Ode.dopri5",
            "Adaptive Dormand–Prince via Ode.dopri5.",
            "lab",
            "sci",
        ),
        live_tool(
            "scientific:num_ode_bdf",
            "BDF",
            "Ode.bdf",
            "Stiff BDF integrator via Ode.bdf.",
            "lab",
            "sci",
        ),
        live_tool(
            "scientific:num_ode_symplectic_step",
            "Symplectic step",
            "Ode.symplectic_step",
            "Verlet/Ruth/Yoshida step via Ode.symplectic_step.",
            "lab",
            "sci",
        ),
    ]
}

pub(super) fn hbbtv_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool(
            "comm:hbbtv_new_app",
            "New HbbTV app",
            "HbbTV.new_app",
            "Create an HbbTV app via HbbTV.new_app (data-hbbtv-id).",
            "comm",
            "comm",
        ),
        live_tool(
            "comm:hbbtv_add_page",
            "Add page",
            "HbbTV.add_page",
            "Add a page via HbbTV.add_page.",
            "comm",
            "comm",
        ),
        live_tool(
            "comm:hbbtv_navigate",
            "Navigate",
            "HbbTV.navigate",
            "Navigate via HbbTV.navigate (data-page-id).",
            "comm",
            "comm",
        ),
        live_tool(
            "comm:hbbtv_set_state",
            "Set state",
            "HbbTV.set_state",
            "Set app state via HbbTV.set_state (data-key).",
            "comm",
            "comm",
        ),
    ]
}
