
/// The maximum size of any discv4 packet is 1280 bytes.
const MAX_PACKET_SIZE: usize = 1280;

/// The minimum size of any discv4 packet. It's the size of the packet header.
const MIN_PACKET_SIZE: usize = 32 + 65 + 1;

/// Default discv4 port, both for UDP and TCP.
const DEFAULT_DISCV4_PORT: u16 = 30303;


