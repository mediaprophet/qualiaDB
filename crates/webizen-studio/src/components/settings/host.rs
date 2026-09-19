use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use webizen_studio::tauri_ffi::invoke as tauri_invoke;

/// Continuity-safe Planned copy when the desktop/Tauri host is not on this surface.
/// Orbit Mail paints this via status.set — Live/Planned honesty only (Capt UAT).
pub const DESKTOP_HOST_PLANNED: &str = "Planned — desktop host not on this surface";

#[cfg(target_arch = "wasm32")]
pub async fn invoke_json<T: DeserializeOwned>(
    command: &str,
    args: serde_json::Value,
) -> Result<T, String> {
    if !crate::endpoints::is_native_host() {
        return Err(DESKTOP_HOST_PLANNED.to_string());
    }
    let args = serde_wasm_bindgen::to_value(&args).map_err(|error| error.to_string())?;
    let result = tauri_invoke(command, args.into())
        .await
        .map_err(|error| format!("{error:?}"))?;
    serde_wasm_bindgen::from_value(result).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke_json<T: DeserializeOwned>(
    _command: &str,
    _args: serde_json::Value,
) -> Result<T, String> {
    Err("Desktop commands are available in the Webizen host".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_host_missing_is_planned_not_held_theatre() {
        assert_eq!(
            DESKTOP_HOST_PLANNED,
            "Planned — desktop host not on this surface"
        );
        assert!(
            DESKTOP_HOST_PLANNED.starts_with("Planned"),
            "missing desktop host must use Planned honesty"
        );
        assert!(
            !DESKTOP_HOST_PLANNED.to_ascii_lowercase().contains("held"),
            "host.rs Planned string must not contain held (Capt orbit Mail UAT)"
        );
        // Production Err path must reference the constant (no inline held theatre).
        let src = include_str!("host.rs");
        let prod: String = src
            .lines()
            .take_while(|l| !l.contains("#[cfg(test)]"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !prod.to_ascii_lowercase().contains("held"),
            "host.rs production source must not contain held"
        );
        assert!(
            prod.contains("DESKTOP_HOST_PLANNED"),
            "invoke_json must return DESKTOP_HOST_PLANNED"
        );
    }
}
