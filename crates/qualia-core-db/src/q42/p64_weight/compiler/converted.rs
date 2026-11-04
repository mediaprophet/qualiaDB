use super::*;
pub(super) fn compile_converted(
    input: &[u8],
    page_log2: u16,
    layout: P64ConvertLayout,
) -> Result<Vec<u8>, String> {
    use etalon_model::p64::convert::{ConversionInput, ConvertLayout};
    let index = GgufTensorIndex::from_gguf(input);
    if index.tensor_data_start == 0 || index.entries.is_empty() {
        return Err("p64: GGUF parse yielded no tensors".into());
    }
    let planned = planning::ordered_tensors(&index)?;
    let names = planned
        .iter()
        .map(|(role, layer, hash, _)| p64_tensor_name(*role, *layer, *hash))
        .collect::<Vec<_>>();
    let mut tokenizer = crate::gguf_sharder::GgufTokenizer::from_gguf(input).to_p64_section();
    native_metadata::append(&index, &mut tokenizer)?;
    let reserved = native_metadata::locator(&index, &tokenizer).map_err(|e| e.to_string())?;
    let mut hp = [0u8; 64];
    let count = index
        .hyperparams
        .n_layer
        .checked_add(1)
        .ok_or("p64: manifold count overflow")? as usize;
    let mut manifolds = vec![
        0;
        count
            .checked_mul(64)
            .ok_or("p64: manifold table overflow")?
    ];
    verbatim::metadata::prepare(&index, &mut hp, &mut manifolds);
    let source = ConversionInput {
        tensor_data_start: index.tensor_data_start as usize,
        tensors: &planned,
        names: &names,
        hparams: P64HParams::read_le(&hp)?,
        tokenizer: &tokenizer,
        header_reserved: reserved,
        manifold_bytes: &manifolds,
    };
    let layout = match layout {
        P64ConvertLayout::F16Expand => ConvertLayout::F16Expand,
        P64ConvertLayout::Q4kSoa => ConvertLayout::Q4kSoa,
        P64ConvertLayout::Verbatim => return super::compile_gguf_to_p64(input, page_log2),
    };
    etalon_model::p64::convert::compile_converted(
        input,
        page_log2,
        layout,
        source,
        Default::default(),
    )
}
