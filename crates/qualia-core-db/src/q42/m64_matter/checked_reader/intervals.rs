//! Caller-buffered iterative heapsort for deterministic bounded overlap checks.
use super::M64PayloadInterval;

fn sift_down(values: &mut [M64PayloadInterval], mut root: usize, end: usize) {
    // root < end / 2 means root has a left child, without overflow in 2*root+1.
    while root < end / 2 {
        let mut child = root * 2 + 1;
        if child + 1 < end && values[child].start < values[child + 1].start {
            child += 1;
        }
        if values[root].start >= values[child].start {
            break;
        }
        values.swap(root, child);
        root = child;
    }
}

pub(super) fn has_overlap(values: &mut [M64PayloadInterval]) -> bool {
    for root in (0..values.len() / 2).rev() {
        sift_down(values, root, values.len());
    }
    for end in (1..values.len()).rev() {
        values.swap(0, end);
        sift_down(values, 0, end);
    }
    values.windows(2).any(|pair| pair[0].end > pair[1].start)
}
