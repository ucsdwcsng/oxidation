use sigmf::{utils::get_root_dir, SigMF};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let path = format!("{}/{}", get_root_dir(), "/Argomoon.sigmf");
    let sigmf = SigMF::load_from_file(&path)?;
    let meta = sigmf.get_meta_by_name("Argomoon")?;

    println!("sigmf: {:?}", sigmf);
    println!("meta: {:#?}", meta);

    Ok(())
}
