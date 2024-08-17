use sigmf::SmfRecording;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let sigmf = SmfRecording::from_file("/home/groberts/dev/github/oxidation/data/Argomoon")?;

    // Print the sigmf object
    println!("{:#?}", sigmf);

    Ok(())
}
