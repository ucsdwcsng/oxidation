use sigmf::{
    datatype::Datatype,
    utils::data::{decode, encode},
};
use std::{hint::black_box, str::FromStr, time::Instant};

fn main() {
    let datatype = Datatype::from_str("ci16_le").expect("valid benchmark datatype");
    let mut input = Vec::with_capacity(1 << 20);
    while input.len() < 1 << 20 {
        input.extend_from_slice(&[0x34, 0x12, 0xcd, 0xab]);
    }

    let decode_started = Instant::now();
    let mut decoded = Vec::new();
    for _ in 0..20 {
        decoded = decode(black_box(&input), &datatype, 1).expect("decode benchmark fixture");
        black_box(&decoded);
    }
    let decode_elapsed = decode_started.elapsed();

    let encode_started = Instant::now();
    let mut encoded = Vec::new();
    for _ in 0..20 {
        encoded = encode(black_box(&decoded), &datatype, 1).expect("encode benchmark fixture");
        black_box(&encoded);
    }
    let encode_elapsed = encode_started.elapsed();
    assert_eq!(encoded, input);
    println!("1 MiB ci16_le x 20: decode={decode_elapsed:?}, encode={encode_elapsed:?}");
}
