//! Small, real browser-surface effects shared by local spec tools.
//!
//! These are deliberately limited to operations CSS can represent honestly.
//! Pixel editing, media transport, exports, and scene mutations do not belong
//! here merely because they have a button.

use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlDocument, HtmlElement};

pub fn apply(
    document: &Document,
    element: &HtmlElement,
    tool_id: &str,
) -> Result<bool, &'static str> {
    if let Some((command, value)) = document_command(tool_id) {
        let applied = document
            .clone()
            .dyn_into::<HtmlDocument>()
            .ok()
            .and_then(|html| match value {
                Some(value) => html
                    .exec_command_with_show_ui_and_value(command, false, value)
                    .ok(),
                None => html.exec_command(command).ok(),
            })
            .unwrap_or(false);
        return if applied {
            Ok(true)
        } else {
            Err("Focus a document editor and select text before using this tool.")
        };
    }
    let Some((property, value)) = style(tool_id) else {
        return Ok(false);
    };
    let styles = element.style();
    let active = styles
        .get_property_value(property)
        .ok()
        .is_some_and(|current| current.trim() == value);
    if active {
        let _ = styles.remove_property(property);
    } else {
        let _ = styles.set_property(property, value);
    }
    Ok(true)
}

fn document_command(tool_id: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match tool_id {
        "office:underline" => ("underline", None),
        "office:strikethrough" => ("strikeThrough", None),
        "office:subscript" => ("subscript", None),
        "office:superscript" => ("superscript", None),
        "office:highlight" => ("backColor", Some("#fef08a")),
        "office:list" => ("insertUnorderedList", None),
        "office:indent" => ("indent", None),
        "office:align_right" => ("justifyRight", None),
        "office:justify" => ("justifyFull", None),
        "office:insert-section" => (
            "insertHTML",
            Some("<section data-poet-section=\"true\"><p><br></p></section>"),
        ),
        "office:insert-table" => (
            "insertHTML",
            Some("<table data-poet-table=\"true\"><tbody><tr><td><br></td><td><br></td></tr><tr><td><br></td><td><br></td></tr></tbody></table>"),
        ),
        "office:insert-page-break" => (
            "insertHTML",
            Some("<div data-poet-page-break=\"true\" style=\"break-after: page\"><br></div>"),
        ),
        _ => return None,
    })
}

fn style(tool_id: &str) -> Option<(&'static str, &'static str)> {
    Some(match tool_id {
        "image:layer-opacity" => ("opacity", "0.85"),
        "image:sharpen" => ("filter", "contrast(1.15)"),
        "image:hue-saturation" => ("filter", "saturate(1.2)"),
        "image:gaussian-blur" => ("filter", "blur(4px)"),
        "image:posterise" => ("filter", "contrast(2.0)"),
        "image:invert-mask" => ("filter", "invert(1)"),
        "image:colour-balance" => ("filter", "sepia(0.3)"),
        "image:curves" => ("filter", "contrast(1.25) brightness(1.05)"),
        "image:levels" => ("filter", "contrast(1.1) brightness(0.95)"),
        "video:add-blur" => ("filter", "blur(2px)"),
        "video:saturation" => ("filter", "saturate(1.2)"),
        "video:lift" => ("filter", "brightness(1.1)"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_honest_browser_styles_are_declared() {
        assert_eq!(style("image:layer-opacity"), Some(("opacity", "0.85")));
        assert_eq!(style("video:add-blur"), Some(("filter", "blur(2px)")));
        assert_eq!(style("image:clone-stamp"), None);
        assert_eq!(style("audio:play"), None);
    }

    #[test]
    fn office_formatting_uses_the_active_document_selection() {
        assert_eq!(
            document_command("office:underline"),
            Some(("underline", None))
        );
        assert_eq!(
            document_command("office:align_right"),
            Some(("justifyRight", None))
        );
        assert!(matches!(
            document_command("office:insert-table"),
            Some(("insertHTML", Some(_)))
        ));
        assert_eq!(document_command("office:insert-citation"), None);
    }
}
