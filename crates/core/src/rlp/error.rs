#[derive(Debug)]
pub enum RLPDecodeError {
    InvalidLength,
    MalformedData,
    MalformedBoolean,
    UnexpectedList,
}
