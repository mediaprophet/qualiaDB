//! Compatibility exports for the canonical Etalon numerical implementation.
pub use qualia_inference_kernel::moe::routing::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_topk_selection_and_softmax() {
        // 10 experts, select top-3
        let logits = [1.0, 0.5, 4.0, 2.5, -1.0, 3.0, 0.0, 5.0, 1.2, 0.8];
        let mut indices = [0u16; 3];
        let mut weights = [0.0f32; 3];

        let count = route_topk(&logits, 3, true, &mut indices, &mut weights).unwrap();
        assert_eq!(count, 3);
        // Top 3 should be: index 7 (5.0), index 2 (4.0), index 5 (3.0)
        assert_eq!(indices, [7, 2, 5]);

        // Sum of weights should equal 1.0
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(weights[0] > weights[1]);
        assert!(weights[1] > weights[2]);
    }

    #[test]
    fn test_qwen_top8_capacity() {
        let mut logits = [0.0f32; 256];
        logits[42] = 10.0;
        logits[100] = 9.0;
        logits[12] = 8.0;
        logits[200] = 7.0;
        logits[1] = 6.0;
        logits[99] = 5.0;
        logits[150] = 4.0;
        logits[255] = 3.0;

        let mut indices = [0u16; 8];
        let mut weights = [0.0f32; 8];

        let count = route_topk(&logits, 8, true, &mut indices, &mut weights).unwrap();
        assert_eq!(count, 8);
        assert_eq!(indices[0], 42);
        assert_eq!(indices[1], 100);
        assert_eq!(indices[2], 12);
        assert_eq!(indices[3], 200);
        assert_eq!(indices[4], 1);
        assert_eq!(indices[5], 99);
        assert_eq!(indices[6], 150);
        assert_eq!(indices[7], 255);
    }

    #[test]
    fn test_combine_expert_outputs() {
        let exp1 = [1.0, 2.0, 3.0];
        let exp2 = [4.0, 5.0, 6.0];
        let outputs = [&exp1[..], &exp2[..]];
        let weights = [0.5, 0.5];
        let mut combined = [0.0f32; 3];

        combine_expert_outputs(&outputs, &weights, 3, &mut combined).unwrap();
        assert_eq!(combined, [2.5, 3.5, 4.5]);
    }
}
