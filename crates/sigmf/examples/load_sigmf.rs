use sigmf::utils::{data, file::get_root_dir};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    // let path = format!("{}/{}", get_root_dir(), "temp/0x0_0.sigmf");
    let save_path = format!("{}/{}", get_root_dir(), "/temp/0x0_0.sigmf");
    println!("save_path: {:?}", save_path);

    let mut sigmf = sigmf::Builder::load(&save_path)?;

    let mut max_num_samples = 0;
    let mut max_meta = String::new();
    for recording in sigmf.get_recordings() {
        let meta = sigmf.get_meta_from_recording(&recording)?;
        let samples = sigmf.get_samples_from_recording(&recording)?;

        if max_num_samples < samples.len() {
            max_num_samples = samples.len();
            max_meta = recording.get_name();
        }
        // println!("num_samples: {:?}", samples);
    }

    println!("max_num_samples: {}", max_num_samples);
    println!("max_meta: {:#?}", max_meta);

    // sigmf.trim(20_000)?;
    // sigmf.save(&save_path)?;

    println!("{:#?}", sigmf);

    Ok(())
}
