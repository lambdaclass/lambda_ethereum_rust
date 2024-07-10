/// Converts a unsigned integer to the next closest multiple of 32
pub fn ceil32(n: u64) -> u64 {
    let rem = n % 32;
    if rem == 0 {
        n
    } else {
        n + 32 - rem
    }
}
