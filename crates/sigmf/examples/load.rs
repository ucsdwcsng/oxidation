use sigmf::{utils::file::get_root_dir, SigMF};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let path = format!("{}/{}", get_root_dir(), "Argomoon.sigmf");
    let sigmf = SigMF::load(&path)?;

    println!("sigmf: {:?}", sigmf);
    Ok(())
}
