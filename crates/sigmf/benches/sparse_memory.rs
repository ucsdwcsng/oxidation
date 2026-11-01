use sigmf::{
    datatype::Datatype, global::Global, meta::Metadata, recording_file::Recording, SigmfError,
};
use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tar::{Builder as TarBuilder, EntryType, Header};

struct TemporaryDirectory(PathBuf);

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct SparseWriter(File);

impl Write for SparseWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.iter().all(|byte| *byte == 0) {
            let end = self
                .0
                .stream_position()?
                .checked_add(u64::try_from(buffer.len()).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
                })?)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "file size overflow"))?;
            if self.0.metadata()?.len() < end {
                self.0.set_len(end)?;
            }
            self.0.seek(SeekFrom::Start(end))?;
            Ok(buffer.len())
        } else {
            self.0.write(buffer)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl Seek for SparseWriter {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.0.seek(position)
    }
}

struct ZeroReader {
    remaining: u64,
}

impl Read for ZeroReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let buffer_length = u64::try_from(buffer.len())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        let count = usize::try_from(self.remaining.min(buffer_length))
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        buffer[..count].fill(0);
        self.remaining -= u64::try_from(count)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        Ok(count)
    }
}

fn main() -> Result<(), SigmfError> {
    let dataset_bytes = std::env::args()
        .nth(1)
        .map(|value| {
            value.parse::<u64>().map_err(|error| {
                SigmfError::InvalidRange(format!("invalid dataset byte count: {error}"))
            })
        })
        .transpose()?
        .unwrap_or(100 * 1024 * 1024);
    let block_frames = 65_536;
    let root = std::env::temp_dir().join(format!(
        "sigmf-sparse-memory-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| SigmfError::InvalidRange(error.to_string()))?
            .as_nanos()
    ));
    fs::create_dir(&root)?;
    let _temporary_directory = TemporaryDirectory(root.clone());

    let metadata_path = root.join("sparse.sigmf-meta");
    let dataset_path = root.join("sparse.sigmf-data");
    let metadata = Metadata::new(
        Global::new("1.2.6", Datatype::from_str("ru8")?),
        Vec::new(),
        Vec::new(),
    );
    fs::write(&metadata_path, metadata.to_vec()?)?;
    File::create(&dataset_path)?.set_len(dataset_bytes)?;

    let mut recording = Recording::open(&metadata_path)?;
    let frame_count = recording.frame_count()?;
    let mut observed_frames = 0u64;
    for block in recording.frame_blocks(0, frame_count, block_frames)? {
        let block_frames_read = u64::try_from(block?.len())
            .map_err(|_| SigmfError::ResourceLimit("block size does not fit u64".into()))?;
        observed_frames = observed_frames
            .checked_add(block_frames_read)
            .ok_or_else(|| SigmfError::ResourceLimit("frame counter overflow".into()))?;
    }
    if observed_frames != frame_count {
        return Err(SigmfError::InvalidRange(format!(
            "read {observed_frames} frames from a dataset containing {frame_count}"
        )));
    }
    println!(
        "Read {observed_frames} frames from a {dataset_bytes}-byte sparse dataset in blocks of {block_frames} frames"
    );
    measure_archive(&root, &metadata, dataset_bytes)?;
    Ok(())
}

fn measure_archive(
    root: &std::path::Path,
    metadata: &Metadata,
    dataset_bytes: u64,
) -> Result<(), SigmfError> {
    let archive_path = root.join("sparse.sigmf");
    let output = SparseWriter(File::create(&archive_path)?);
    let mut builder = TarBuilder::new(output);

    let metadata_bytes = metadata.to_vec()?;
    let mut metadata_header = Header::new_gnu();
    metadata_header.set_entry_type(EntryType::Regular);
    metadata_header.set_size(u64::try_from(metadata_bytes.len()).map_err(|error| {
        SigmfError::ResourceLimit(format!("metadata size does not fit u64: {error}"))
    })?);
    metadata_header.set_mode(0o644);
    metadata_header.set_cksum();
    builder.append_data(
        &mut metadata_header,
        "sparse.sigmf-meta",
        metadata_bytes.as_slice(),
    )?;

    let mut dataset_header = Header::new_gnu();
    dataset_header.set_entry_type(EntryType::Regular);
    dataset_header.set_size(dataset_bytes);
    dataset_header.set_mode(0o644);
    dataset_header.set_cksum();
    builder.append_data(
        &mut dataset_header,
        "sparse.sigmf-data",
        ZeroReader {
            remaining: dataset_bytes,
        },
    )?;
    builder.finish()?;
    drop(builder);

    let archive_path = archive_path.to_string_lossy();
    let archive = sigmf::Archive::load(&archive_path)?;
    if archive.recordings.len() != 1 {
        return Err(SigmfError::InvalidMetadata(format!(
            "expected one recording in the sparse archive, found {}",
            archive.recordings.len()
        )));
    }
    println!("Indexed one recording in a {dataset_bytes}-byte sparse dataset archive");
    Ok(())
}
