//! Closest-match suggestions for unknown catalog ids.

use super::ALL_INVOKE_IDS;
use crate::animation::presets::list_all_presets;

pub fn did_you_mean(path: &str) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for id in ALL_INVOKE_IDS.iter().copied() {
        let d = edit_distance(path, id);
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((id, d));
        }
    }
    for info in list_all_presets() {
        let alias = format!("Animation.{}", info.preset);
        let d = edit_distance(path, &alias);
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            // leak-free: only keep if we later format from a static id
            if d <= 4 {
                return Some(alias);
            }
        }
    }
    let (id, d) = best?;
    if d <= 6 && d * 3 < path.len().max(id.len()) + 6 {
        Some(id.to_string())
    } else {
        None
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let aa: Vec<char> = a.chars().collect();
    let bb: Vec<char> = b.chars().collect();
    let n = aa.len();
    let m = bb.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = if aa[i - 1] == bb[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}
