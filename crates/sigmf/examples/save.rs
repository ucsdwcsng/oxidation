use sigmf::{utils::file::get_root_dir, SigMF};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let save_path = format!("{}/{}", get_root_dir(), "Argomoon_save.sigmf");
    let sigmf = SigMF::new(save_path);

    sigmf.save()?;
    Ok(())
}

