# Model Compression

`specialized_libs::machine_learning::ModelCompression` provides a
model-container-independent compression layer over flat `f64` weight tensors.
It deliberately does not depend on GGUF: GGUF-specific Q4/Q8 codecs remain in
the inference subsystem.

## Post-training quantization

`quantize_symmetric_int8_into` performs per-tensor symmetric int8 PTQ:

```text
scale = max(abs(weights)) / 127
q[i]  = clamp(round(weights[i] / scale), -127, 127)
```

The caller supplies the `i8` payload buffer. `QuantizationReport` records the
actual byte ratio (including the scale), RMSE, and maximum absolute
reconstruction error. `dequantize_symmetric_int8_into` reconstructs into a
caller-owned `f64` slice.

This is PTQ, not quantization-aware training. QAT requires fake-quantized
forward/backward operators that the current single-linear-layer trainer does
not provide.

## Pruning

Two deterministic magnitude policies are implemented:

- `prune_unstructured_into` removes individual weights with the smallest
  absolute magnitude.
- `prune_output_channels_into` removes complete row-major output channels with
  the smallest squared L2 norm.

Both methods produce an actual sparse representation: a one-bit keep mask and
packed retained values in original order. Sorting workspace, masks, packed
values, and reconstructed output are supplied by the caller. Equal-magnitude
ties are resolved by original index.

`TrainingEngine::start_training_with_pruning_mask` supports recovery training
for an unstructured mask. Removed weights are zeroed before training and are
never updated, preventing accidental regrowth.

## Knowledge distillation

`distill_linear_student` executes the supported teacher-student loop:

1. Run the teacher through the real MLP inference path for every sample.
2. Optionally blend teacher outputs with caller-provided hard targets.
3. Train a single-linear-layer student through the existing SGD/MSE backend.
4. Measure teacher/student fidelity before and after training.

The teacher may be a larger multi-layer MLP supported by `InferenceEngine`.
The student is intentionally restricted to the trainer's current single linear
layer. Classification-logit distillation, temperature-scaled KL loss, CNN or
transformer fine-tuning, and QAT remain future training-infrastructure work.

## Evidence and limits

Every successful operation updates measured byte ratios and a bounded quality
proxy:

- PTQ: RMSE and maximum reconstruction error.
- Pruning: achieved sparsity and retained L2 energy.
- Distillation: teacher/student MSE before and after SGD.

These metrics establish numerical correctness and compression/fidelity
trade-offs. They are not a substitute for task-specific validation accuracy or
hardware latency benchmarks on a production model.
