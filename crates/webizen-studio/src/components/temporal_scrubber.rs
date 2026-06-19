use dioxus::prelude::*;

/// Temporal scrubber for time-trail navigation
///
/// Allows navigation through temporal slices (t values) of the knowledge graph.
/// Filters nodes by version <= current_t.
///
/// Zero-heap considerations:
/// - Component state uses heap allocation (inherent to Dioxus/React framework)
/// - f64 values are stack-allocated in the component state
/// - The actual temporal filtering happens in backend daemon (zero-heap compliant)
#[component]
pub fn TemporalScrubber(
    current_t: Signal<f64>,
    min_t: f64,
    max_t: f64,
    on_scrub: Callback<f64>,
) -> Element {
    let step = (max_t - min_t) / 100.0; // 100 steps across the range

    rsx! {
        div {
            class: "temporal-scrubber",
            style: "display: flex; align-items: center; gap: 12px; padding: 12px; background: rgba(0, 0, 0, 0.3); border-radius: 8px;",

            span {
                style: "font-size: 12px; color: #888;",
                "Time Slice: {current_t():.2}"
            }

            input {
                r#type: "range",
                min: "{min_t}",
                max: "{max_t}",
                step: "{step}",
                value: "{current_t()}",
                style: "flex: 1; cursor: pointer;",
                oninput: move |evt| {
                    if let Ok(value) = evt.value().parse::<f64>() {
                        current_t.set(value);
                        on_scrub.call(value);
                    }
                }
            }

            span {
                style: "font-size: 10px; color: #666;",
                "Min: {min_t:.0}"
            }

            span {
                style: "font-size: 10px; color: #666;",
                "Max: {max_t:.0}"
            }
        }
    }
}
