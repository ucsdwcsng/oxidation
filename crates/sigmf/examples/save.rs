use sigmf::{global, meta::SmfMeta, utils::file::get_root_dir, Number};
use std::error::Error;

const SAVE_PATH: &str = "Argomoon_save.sigmf";
const AUTHOR: &str = r#"Daniel Est\u00e9vez <daniel@destevez.net>"#;
const DESCRIPTION: &str =
    "Recording of Artemis I Argomoon with the Allen Telescope Array on 2022-11-16";
const HW: &str = "ATA antenna 1a, LO d, polarization X, USRP N32x";
const RECORDER: &str = "GNU Radio 3.10";
const LICENSE: &str = "https://creativecommons.org/licenses/by/4.0/";
const VERSION: &str = "1.0.0";
const SAMPLE_RATE: Number = 1920000.0;

fn main() -> Result<(), Box<dyn Error>> {
    let path = format!("{}/{}", get_root_dir(), SAVE_PATH);
    let mut builder = sigmf::Builder::new(path);
    let mut meta = SmfMeta::default();
    meta.set_author(AUTHOR.to_string());
    meta.set_datatype(global::SmfDataType::new(
        global::SmfDataTypeDomain::Complex,
        Some(global::SmfDataTypeDtype::I16),
        Some(global::SmfDataTypeEndian::LE),
        None,
    ));
    meta.set_description(DESCRIPTION.to_string());
    meta.set_hw(HW.to_string());
    meta.set_license(LICENSE.to_string());
    meta.set_recorder(RECORDER.to_string());
    meta.set_version(VERSION.to_string());
    meta.set_sample_rate(SAMPLE_RATE);

    let filename = "searchlight";

    builder.add_entry(filename, meta)?;

    let sigmf = builder.build()?;

    println!("sigmf: {:#?}", sigmf);
    Ok(())
}
