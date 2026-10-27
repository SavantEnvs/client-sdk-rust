/// Splits the input into the selector byte and the remaining payload.
pub fn split_selector(data: &[u8]) -> Option<(u8, &[u8])> {
    data.split_first().map(|(s, rest)| (*s, rest))
}
