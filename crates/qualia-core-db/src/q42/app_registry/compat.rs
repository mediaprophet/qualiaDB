//! Simple engine-version compatibility helpers.
//!
//! # Limitations
//!
//! Comparison is **not** full SemVer: pre-release / build metadata (`-rc.1`,
//! `+git`) are treated as opaque trailing text on the last numeric segment.
//! Components are split on `.`; numeric parts compare as `u64`, non-numeric as
//! byte strings. Missing trailing components are treated as `0`.

/// Returns `true` when `min_required` is strictly greater than `engine`
/// under the simple dotted rules above (→ mark record [`Incompatible`](super::record::AppRecordState::Incompatible)).
pub fn engine_too_old(min_required: &str, engine: &str) -> bool {
    compare_simple_version(min_required, engine) == std::cmp::Ordering::Greater
}

/// Returns `true` when `engine` is strictly greater than `max_allowed`
/// (non-empty upper bound). Empty `max_allowed` means no upper bound.
pub fn engine_too_new(max_allowed: &str, engine: &str) -> bool {
    if max_allowed.trim().is_empty() {
        return false;
    }
    compare_simple_version(engine, max_allowed) == std::cmp::Ordering::Greater
}

fn compare_simple_version(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.trim().split('.').collect();
    let b_parts: Vec<&str> = b.trim().split('.').collect();
    let n = a_parts.len().max(b_parts.len());
    for i in 0..n {
        let left = a_parts.get(i).copied().unwrap_or("0");
        let right = b_parts.get(i).copied().unwrap_or("0");
        match (parse_leading_u64(left), parse_leading_u64(right)) {
            (Some(l), Some(r)) => {
                let ord = l.cmp(&r);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
                // Equal numeric prefix — compare residual suffix if any.
                let l_rest = &left[digit_prefix_len(left)..];
                let r_rest = &right[digit_prefix_len(right)..];
                let so = l_rest.cmp(r_rest);
                if so != std::cmp::Ordering::Equal {
                    return so;
                }
            }
            _ => {
                let ord = left.cmp(right);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
        }
    }
    std::cmp::Ordering::Equal
}

fn parse_leading_u64(s: &str) -> Option<u64> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn digit_prefix_len(s: &str) -> usize {
    s.chars().take_while(|c| c.is_ascii_digit()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_ordering() {
        assert!(engine_too_old("0.0.37", "0.0.36"));
        assert!(!engine_too_old("0.0.36", "0.0.36"));
        assert!(!engine_too_old("0.0.35", "0.0.36"));
        assert!(engine_too_old("1.0.0", "0.9.9"));
    }

    #[test]
    fn upper_bound() {
        assert!(!engine_too_new("", "9.9.9"));
        assert!(!engine_too_new("1.0.0", "1.0.0"));
        assert!(engine_too_new("0.0.36", "0.0.37"));
    }
}
