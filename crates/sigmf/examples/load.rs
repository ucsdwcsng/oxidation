use sigmf::SmfRecording;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let path = "/home/groberts/dev/github/oxidation/data/Argomoon";
    let data_path = path.to_string() + ".sigmf-data";
    let meta_path = path.to_string() + ".sigmf-meta";

    let recording = SmfRecording::new(&meta_path, &data_path)?;
    println!("{:?}", recording);

    Ok(())
}
