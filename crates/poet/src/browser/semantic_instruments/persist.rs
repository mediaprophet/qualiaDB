//! Compact string codec for instrument-graph centres.
//!
//! Format: `"x,y;x,y;..."` at two decimal places. No Document, no Host IDs.

/// Encode centres as `"x,y;x,y;..."` with two decimal places.
pub fn encode_positions(pts: &[(f32, f32)]) -> String {
    let mut out = String::new();
    for (i, &(x, y)) in pts.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push_str(&format!("{x:.2},{y:.2}"));
    }
    out
}

/// Decode centres. Empty → empty. Skip malformed pairs and any token containing `Host`.
pub fn decode_positions(s: &str) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for token in s.split(';') {
        let token = token.trim();
        if token.is_empty() || token.contains("Host") {
            continue;
        }
        let Some((xs, ys)) = token.split_once(',') else {
            continue;
        };
        if ys.contains(',') {
            continue;
        }
        let Ok(x) = xs.trim().parse::<f32>() else {
            continue;
        };
        let Ok(y) = ys.trim().parse::<f32>() else {
            continue;
        };
        if x.is_finite() && y.is_finite() {
            out.push((x, y));
        }
    }
    out
}

/// True when `decode(encode(pts))` matches each coordinate within `0.02`.
pub fn round_trip_ok(pts: &[(f32, f32)]) -> bool {
    let back = decode_positions(&encode_positions(pts));
    back.len() == pts.len()
        && pts
            .iter()
            .zip(back.iter())
            .all(|(a, b)| (a.0 - b.0).abs() <= 0.02 && (a.1 - b.1).abs() <= 0.02)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_empty() {
        assert_eq!(encode_positions(&[]), "");
        assert!(decode_positions("").is_empty());
        assert!(decode_positions("   ").is_empty());
        assert!(decode_positions(";;").is_empty());
        assert!(round_trip_ok(&[]));
    }

    #[test]
    fn two_points_round_trip() {
        let pts = [(12.345, 67.891), (0.0, -3.2)];
        assert_eq!(encode_positions(&pts), "12.35,67.89;0.00,-3.20");
        assert!(round_trip_ok(&pts));
        let back = decode_positions(&encode_positions(&pts));
        assert_eq!(back.len(), 2);
        assert!((back[0].0 - 12.35).abs() < 0.001);
        assert!((back[1].1 + 3.20).abs() < 0.001);
    }

    #[test]
    fn junk_pairs_are_ignored() {
        let got = decode_positions("foo;1.00,2.00;bar,baz;3.5;4.00,5.00,6.00;;7.25,8.50");
        assert_eq!(got, vec![(1.00, 2.00), (7.25, 8.50)]);
    }

    #[test]
    fn host_in_string_does_not_become_a_node() {
        let got = decode_positions("10.00,20.00;Host.1,2.00;Host.;30.00,40.00;Host.assess");
        assert_eq!(got, vec![(10.00, 20.00), (30.00, 40.00)]);
        assert!(!encode_positions(&[(1.0, 2.0)]).contains("Host"));
    }
}
