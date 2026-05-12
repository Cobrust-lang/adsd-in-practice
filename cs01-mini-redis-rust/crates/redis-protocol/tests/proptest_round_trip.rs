//! Property-based round-trip tests for redis-protocol.
//!
//! Verifies `parse(to_bytes(f)).0 == f` for randomly generated frames.
//! Uses proptest with 256 cases (≥ 100 required by ADR-0002 §Done Criteria).
//!
//! ADSD note: ≥ 1,000 fuzz inputs is P0 target; 256 is M1.1 baseline.
//! True 1,000+ coverage lands at M2 per task spec.

use proptest::prelude::*;
use redis_protocol::Frame;

/// Recursive strategy for generating arbitrary RESP v2 frames.
/// Depth-limited to avoid excessive recursion / test slowness.
fn frame_strategy() -> impl Strategy<Value = Frame> {
    let leaf = prop_oneof![
        // SimpleString — printable ASCII, no \r or \n
        "[a-zA-Z0-9 _\\-]{0,64}".prop_map(Frame::SimpleString),
        // Error — same safe charset
        "[a-zA-Z0-9 _\\-]{0,64}".prop_map(Frame::Error),
        // Integer — full i64 range
        any::<i64>().prop_map(Frame::Integer),
        // BulkString(Some) — arbitrary bytes
        prop::collection::vec(any::<u8>(), 0..64).prop_map(|v| Frame::BulkString(Some(v))),
        // BulkString(None)
        Just(Frame::BulkString(None)),
        // Array(None)
        Just(Frame::Array(None)),
    ];

    leaf.prop_recursive(
        3,  // max depth
        64, // max total nodes in the tree
        4,  // max children per internal node
        |inner| prop::collection::vec(inner, 0..5).prop_map(|v| Frame::Array(Some(v))),
    )
}

proptest! {
    // Run 256 cases (≥ 100 required; bump to 1000 in M2).
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn round_trip(frame in frame_strategy()) {
        let bytes = frame.to_bytes();
        let (parsed, consumed) = Frame::parse(&bytes)
            .expect("parse must succeed on well-formed to_bytes output");
        prop_assert_eq!(parsed, frame);
        prop_assert_eq!(consumed, bytes.len());
    }
}
