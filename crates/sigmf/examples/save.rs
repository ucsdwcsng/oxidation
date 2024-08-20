use sigmf::{
    global,
    meta::SmfMeta,
    utils::{file::get_root_dir, hash::hash_from_slice},
    Number,
};
use std::error::Error;

const FILENAME: &str = "r23-x300";
const AUTHOR: &str = "Gavin Roberts (gsroberts@ucsd.edu)";
const DESCRIPTION: &str = "UHD recording of Atkinson Hall 1601 on 2024-08-19";
const HW: &str = "Ettus Research USRP X300; Radio 23";
const RECORDER: &str = "GNU Radio 3.10";
const LICENSE: &str = "https://creativecommons.org/licenses/by/4.0/";
const VERSION: &str = "1.0.0";
const SAMPLE_RATE: Number = 100e6;

fn main() -> Result<(), Box<dyn Error>> {
    let path = format!("{}/{}.sigmf", get_root_dir(), FILENAME);
    let mut builder = sigmf::Builder::new(path);

    let data = vec![0u8; 1000];
    let data_hash = hash_from_slice(&data)?;

    let mut meta = SmfMeta::default();
    meta.set_sha512(data_hash);
    meta.set_author(AUTHOR.to_string());
    meta.set_datatype(global::SmfDataType::new(
        global::SmfDataTypeDomain::Complex,
        Some(global::SmfDataTypeDtype::F32),
        Some(global::SmfDataTypeEndian::LE),
        None,
    ));
    meta.set_description(DESCRIPTION.to_string());
    meta.set_hw(HW.to_string());
    meta.set_license(LICENSE.to_string());
    meta.set_recorder(RECORDER.to_string());
    meta.set_version(VERSION.to_string());
    meta.set_sample_rate(SAMPLE_RATE);

    builder.add(FILENAME, meta, &data[..])?;

    let sigmf = builder.build()?;
    println!("sigmf: {:#?}", sigmf);
    Ok(())
}
