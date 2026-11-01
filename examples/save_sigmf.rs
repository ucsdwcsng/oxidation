use sigmf::{
    Number,
    global::{self, Global},
    meta::Metadata,
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
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: save_sigmf <output.sigmf>")?;
    let mut builder = sigmf::ArchiveWriter::new(path)?;

    let data = vec![0u8; 1000];

    let datatype = global::Datatype::new(
        global::SampleDomain::Complex,
        Some(global::ComponentType::F32),
        Some(global::Endianness::LE),
        None,
    )?;
    let mut meta = Metadata::new(Global::new(VERSION, datatype), Vec::new(), Vec::new());
    meta.set_author(AUTHOR.to_string());
    meta.set_description(DESCRIPTION.to_string());
    meta.set_hw(HW.to_string());
    meta.set_license(LICENSE.to_string());
    meta.set_recorder(RECORDER.to_string());
    meta.set_sample_rate(SAMPLE_RATE);

    builder.add(FILENAME, meta, &data[..])?;

    let sigmf = builder.build()?;
    println!("sigmf: {:#?}", sigmf);
    Ok(())
}
