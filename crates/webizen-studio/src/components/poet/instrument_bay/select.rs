//! Pure selection helper for instrument flow rails.
//!
//! Maps a flow-node label to a constraint rail and filters which shape
//! groups the chrome should show. No Host IDs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rail {
    Pipeline,
    InputShape,
    OutputShape,
}

/// Classify a flow-node label. Unknown labels stay on the pipeline rail.
pub fn rail_for_flow_label(label: &str) -> Rail {
    match label.trim().to_ascii_lowercase().as_str() {
        "input shape" => Rail::InputShape,
        "output shape" => Rail::OutputShape,
        "assess" | "recognise" | "entry" | "logic" | "n3/cml logic" => Rail::Pipeline,
        _ => Rail::Pipeline,
    }
}

/// `(show_input, show_output)` for the selected rail. Empty groups stay hidden.
pub fn filter_constraints(rail: Rail, input_n: usize, output_n: usize) -> (bool, bool) {
    let has_in = input_n > 0;
    let has_out = output_n > 0;
    match rail {
        Rail::Pipeline => (has_in, has_out),
        Rail::InputShape => (has_in, false),
        Rail::OutputShape => (false, has_out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_map_to_rails() {
        assert_eq!(rail_for_flow_label("assess"), Rail::Pipeline);
        assert_eq!(rail_for_flow_label("recognise"), Rail::Pipeline);
        assert_eq!(rail_for_flow_label("entry"), Rail::Pipeline);
        assert_eq!(rail_for_flow_label("input shape"), Rail::InputShape);
        assert_eq!(rail_for_flow_label("output shape"), Rail::OutputShape);
        assert_eq!(rail_for_flow_label("N3/CML logic"), Rail::Pipeline);
        assert_eq!(rail_for_flow_label("logic"), Rail::Pipeline);
        assert_eq!(rail_for_flow_label("  Input Shape  "), Rail::InputShape);
    }

    #[test]
    fn host_dot_label_defaults_to_pipeline() {
        assert_eq!(rail_for_flow_label("Host."), Rail::Pipeline);
        assert_eq!(rail_for_flow_label("Host.assess"), Rail::Pipeline);
    }

    #[test]
    fn filter_input_shape_shows_only_input() {
        assert_eq!(filter_constraints(Rail::InputShape, 2, 3), (true, false));
        assert_eq!(filter_constraints(Rail::InputShape, 0, 3), (false, false));
        assert_eq!(filter_constraints(Rail::OutputShape, 2, 3), (false, true));
        assert_eq!(filter_constraints(Rail::Pipeline, 2, 3), (true, true));
        assert_eq!(filter_constraints(Rail::Pipeline, 0, 1), (false, true));
    }
}
