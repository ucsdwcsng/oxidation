use sigmf::{utils::file::get_root_dir, SigMF};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let mut sigmf = SigMF::default();
    let save_path = format!("{}/{}", get_root_dir(), "Argomoon_save.sigmf");

    sigmf.set_path(&save_path);

    sigmf.save_to_file(&save_path)?;

    Ok(())
}
