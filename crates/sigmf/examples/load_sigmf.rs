use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: load_sigmf <archive.sigmf>")?;
    let archive = sigmf::Archive::load(&path)?;
    for recording in archive.get_recordings() {
        let metadata = archive.get_meta_from_recording(&recording)?;
        println!("{}: {:?}", recording.get_name(), metadata.global().datatype);
    }
    Ok(())
}
