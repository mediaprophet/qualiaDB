//! Fixed Add graph encoded from https://github.com/onnx/onnx/blob/main/onnx/onnx.proto.

pub const ELEMENTS: usize = 16;

fn varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn integer(field: u64, value: u64, output: &mut Vec<u8>) {
    varint(field << 3, output);
    varint(value, output);
}

fn bytes(field: u64, value: &[u8], output: &mut Vec<u8>) {
    varint((field << 3) | 2, output);
    varint(value.len() as u64, output);
    output.extend_from_slice(value);
}

fn tensor_info(name: &[u8]) -> Vec<u8> {
    let mut shape = Vec::new();
    for dimension in [1, ELEMENTS as u64] {
        let mut dim = Vec::new();
        integer(1, dimension, &mut dim);
        bytes(1, &dim, &mut shape);
    }
    let mut tensor_type = Vec::new();
    integer(1, 1, &mut tensor_type);
    bytes(2, &shape, &mut tensor_type);
    let mut type_info = Vec::new();
    bytes(1, &tensor_type, &mut type_info);
    let mut info = Vec::new();
    bytes(1, name, &mut info);
    bytes(2, &type_info, &mut info);
    info
}

pub fn add_model() -> Vec<u8> {
    let mut node = Vec::new();
    bytes(1, b"left", &mut node);
    bytes(1, b"right", &mut node);
    bytes(2, b"sum", &mut node);
    bytes(3, b"qualia_npu_add", &mut node);
    bytes(4, b"Add", &mut node);
    let mut graph = Vec::new();
    bytes(1, &node, &mut graph);
    bytes(2, b"qualia_npu_arithmetic", &mut graph);
    bytes(11, &tensor_info(b"left"), &mut graph);
    bytes(11, &tensor_info(b"right"), &mut graph);
    bytes(12, &tensor_info(b"sum"), &mut graph);
    let mut opset = Vec::new();
    integer(2, 13, &mut opset);
    let mut model = Vec::new();
    integer(1, 8, &mut model);
    bytes(2, b"qualia-accelerator-harness", &mut model);
    bytes(7, &graph, &mut model);
    bytes(8, &opset, &mut model);
    model
}
