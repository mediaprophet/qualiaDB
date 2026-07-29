#[derive(Debug, Clone)]
pub struct TxIn {
    pub prev_txid: String,
    pub prev_out_idx: u32,
    pub signature_script: Vec<u8>,
    pub sequence: u32,
}

#[derive(Debug, Clone)]
pub struct TxOut {
    pub value: u64,
    pub pk_script: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: i32,
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    pub lock_time: u32,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.extend_from_slice(&self.version.to_le_bytes());

        buf.extend_from_slice(&encode_varint(self.inputs.len() as u64));
        for input in &self.inputs {
            let mut txid_bytes = hex::decode(&input.prev_txid).unwrap_or_else(|_| vec![0; 32]);
            txid_bytes.reverse();
            buf.extend_from_slice(&txid_bytes);

            buf.extend_from_slice(&input.prev_out_idx.to_le_bytes());

            buf.extend_from_slice(&encode_varint(input.signature_script.len() as u64));
            buf.extend_from_slice(&input.signature_script);

            buf.extend_from_slice(&input.sequence.to_le_bytes());
        }

        buf.extend_from_slice(&encode_varint(self.outputs.len() as u64));
        for output in &self.outputs {
            buf.extend_from_slice(&output.value.to_le_bytes());
            buf.extend_from_slice(&encode_varint(output.pk_script.len() as u64));
            buf.extend_from_slice(&output.pk_script);
        }

        buf.extend_from_slice(&self.lock_time.to_le_bytes());

        buf
    }
}

pub fn encode_varint(val: u64) -> Vec<u8> {
    if val < 0xfd {
        vec![val as u8]
    } else if val <= 0xffff {
        let mut res = vec![0xfd];
        res.extend_from_slice(&(val as u16).to_le_bytes());
        res
    } else if val <= 0xffffffff {
        let mut res = vec![0xfe];
        res.extend_from_slice(&(val as u32).to_le_bytes());
        res
    } else {
        let mut res = vec![0xff];
        res.extend_from_slice(&val.to_le_bytes());
        res
    }
}
