use sigmf::recording_file::Recording;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: load_recording <recording.sigmf-meta>")?;
    let mut recording = Recording::open(path)?;
    let frame_count = recording.frame_count()?;
    let start = recording.first_sample_index();
    for block in recording.frame_blocks(start, frame_count, 4096)? {
        let _samples = block?;
    }

    println!("frames: {frame_count}");
    Ok(())
}
