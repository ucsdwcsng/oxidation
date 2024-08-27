use sigmf::utils::file::get_root_dir;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load a SIGMF file
    let path = format!("{}/{}", get_root_dir(), "temp/0x0_0_trimmed.sigmf");
    // let save_path = format!("{}/{}", get_root_dir(), "temp/0x0_0_trimmed.sigmf");
    let mut sigmf = sigmf::Builder::load(&path)?;

    // sigmf.trim(1000)?;
    // sigmf.save(&save_path)?;

    println!("{:#?}", sigmf);

    Ok(())
}
