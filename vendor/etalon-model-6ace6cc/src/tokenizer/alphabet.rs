const fn direct(b: u8) -> bool {
    (b >= 33 && b <= 126) || (b >= 161 && b <= 172) || b >= 174
}
const fn alphabet() -> [char; 256] {
    let mut table = ['\0'; 256];
    let mut i = 0;
    let mut extra = 256u32;
    while i < 256 {
        let code = if direct(i as u8) {
            i as u32
        } else {
            let c = extra;
            extra += 1;
            c
        };
        table[i] = match char::from_u32(code) {
            Some(c) => c,
            None => '\0',
        };
        i += 1;
    }
    table
}
pub(super) const ALPHABET: [char; 256] = alphabet();
pub(super) fn decode(c: char) -> Option<u8> {
    ALPHABET
        .iter()
        .position(|&candidate| candidate == c)
        .map(|i| i as u8)
}
