//! Render Live Tool Chest chain — Host-bound `Render.*` dual-path tools.

use super::*;

fn render_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "media".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "hm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn render_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        render_live_tool(
            "render:live_scene",
            "Scene",
            "Render.scene",
            "Build a renderer contract via Render.scene (data-scene-kind).",
        ),
        render_live_tool(
            "render:live_css_animation",
            "CSS animation",
            "Render.css_animation",
            "Generate @keyframes CSS via Render.css_animation.",
        ),
        render_live_tool(
            "render:live_css_color",
            "CSS color",
            "Render.css_color",
            "Map EMF α/μ/σ to CSS rgb via Render.css_color.",
        ),
        render_live_tool(
            "render:live_css_transform",
            "CSS transform",
            "Render.css_transform",
            "Build a CSS transform string via Render.css_transform.",
        ),
        render_live_tool(
            "render:live_animation_eval_curve",
            "Eval curve",
            "Render.animation_eval_curve",
            "Evaluate an easing curve via Render.animation_eval_curve.",
        ),
        render_live_tool(
            "render:live_animation_spring_step",
            "Spring step",
            "Render.animation_spring_step",
            "Step a spring via Render.animation_spring_step.",
        ),
        render_live_tool(
            "render:live_animation_sclerp",
            "ScLERP",
            "Render.animation_sclerp",
            "Screw-linear interpolate motors via Render.animation_sclerp.",
        ),
        render_live_tool(
            "render:live_animation_eval_preset",
            "Eval preset",
            "Render.animation_eval_preset",
            "Evaluate an animation preset via Render.animation_eval_preset.",
        ),
        render_live_tool(
            "render:live_animation_squad_step",
            "SQUAD step",
            "Render.animation_squad_step",
            "Squad-interpolate quaternions via Render.animation_squad_step.",
        ),
        render_live_tool(
            "render:live_animation_list_presets",
            "List presets",
            "Render.animation_list_presets",
            "List animation presets via Render.animation_list_presets.",
        ),
        render_live_tool(
            "render:live_animation_compute_pass",
            "Compute pass",
            "Render.animation_compute_pass",
            "Generate a GPU animation compute pass via Render.animation_compute_pass.",
        ),
        render_live_tool(
            "render:live_svg_path",
            "SVG path",
            "Render.svg_path",
            "Generate an SVG path via Render.svg_path.",
        ),
        render_live_tool(
            "render:live_svg_circle",
            "SVG circle",
            "Render.svg_circle",
            "Generate an SVG circle via Render.svg_circle.",
        ),
        render_live_tool(
            "render:live_svg_rect",
            "SVG rect",
            "Render.svg_rect",
            "Generate an SVG rect via Render.svg_rect.",
        ),
        render_live_tool(
            "render:live_svg_line",
            "SVG line",
            "Render.svg_line",
            "Generate an SVG line via Render.svg_line.",
        ),
        render_live_tool(
            "render:live_svg_bezier",
            "SVG bezier",
            "Render.svg_bezier",
            "Sample a Bezier into an SVG path via Render.svg_bezier.",
        ),
        render_live_tool(
            "render:live_svg_field",
            "SVG field",
            "Render.svg_field",
            "Visualise a 2D field via Render.svg_field.",
        ),
    ]
}
