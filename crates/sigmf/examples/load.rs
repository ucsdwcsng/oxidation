use sigmf::{utils::file::get_root_dir, SigMF};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let path = format!("{}/{}", get_root_dir(), "temp/0x0_0.sigmf");
    let sigmf = sigmf::Builder::load(&path)?;

    let recordings = sigmf.get_recordings();
    for recording in &recordings {
        println!("recording: {:?}", recording);
        let meta = match sigmf.get_meta_by_recording(&recording) {
            Ok(meta) => meta,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };
        println!("meta: {:?}", meta);
        let samples = match sigmf.get_samples_by_recording(&recording) {
            Ok(samples) => samples,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };
        println!("samples: {:?}", samples);
    }

    Ok(())
}
