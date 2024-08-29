use sigmf::utils::file::get_root_dir;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a sigmf-meta file
    let path = format!("{}/{}", get_root_dir(), "example.sigmf-data");

    let sigmf = sigmf::Builder::from_recording(&path)?;
    let recording = &sigmf.get_recordings()[0];
    let samples = sigmf.get_samples_from_recording(recording)?;

    println!("samples: {:?}", samples.len());
    Ok(())
}
