#[derive(Debug)]
pub enum RLPDecodeError {
    InvalidLength,
    MalformedData,
}
