use apate::transport::{
    Frame, FrameError, FrameType, MAX_FRAME_PAYLOAD_LEN, decode_frame, encode_frame,
};
use proptest::prelude::*;

fn frame_type_strategy() -> impl Strategy<Value = FrameType> {
    prop_oneof![
        Just(FrameType::Handshake),
        Just(FrameType::Data),
        Just(FrameType::Ack),
        Just(FrameType::Rekey),
        Just(FrameType::Migrate),
        Just(FrameType::Close),
    ]
}

proptest! {
    #[test]
    fn frame_roundtrip_property(
        frame_type in frame_type_strategy(),
        sequence in any::<u64>(),
        payload in proptest::collection::vec(any::<u8>(), 0..=1024),
        flags in 0_u8..=0b0000_0011,
    ) {
        let frame = Frame {
            frame_type,
            sequence,
            payload,
        };

        let encoded = encode_frame(&frame, flags).expect("encode should succeed");
        let decoded = decode_frame(&encoded).expect("decode should succeed");

        prop_assert_eq!(frame, decoded.frame);
        prop_assert_eq!(flags, decoded.context.flags);
        prop_assert_eq!(encoded.len(), decoded.context.total_len);
    }
}

#[test]
fn frame_decode_rejects_malformed_inputs() {
    assert_eq!(Err(FrameError::Malformed), decode_frame(&[1, 2, 3]));

    let mut invalid_type = vec![9, 0, 0, 0];
    invalid_type.extend_from_slice(&0_u64.to_be_bytes());
    assert_eq!(Err(FrameError::UnsupportedType), decode_frame(&invalid_type));

    let mut invalid_flags = vec![1, 0b1000_0000, 0, 0];
    invalid_flags.extend_from_slice(&0_u64.to_be_bytes());
    assert_eq!(Err(FrameError::InvalidFlags), decode_frame(&invalid_flags));

    let mut length_mismatch = vec![1, 0, 0, 4];
    length_mismatch.extend_from_slice(&1_u64.to_be_bytes());
    assert_eq!(Err(FrameError::Malformed), decode_frame(&length_mismatch));

    let too_large_len = u16::try_from(MAX_FRAME_PAYLOAD_LEN + 1).expect("max len fits u16");
    let mut too_large = vec![1, 0];
    too_large.extend_from_slice(&too_large_len.to_be_bytes());
    too_large.extend_from_slice(&0_u64.to_be_bytes());
    assert_eq!(Err(FrameError::PayloadTooLarge), decode_frame(&too_large));
}
