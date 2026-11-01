use crate::meta::Metadata;
use crate::utils::data::{self, SampleValue, SampleWriter};
use crate::{Integer, SigmfError};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

struct DatasetLayout {
    frame_count: Integer,
    bytes_per_frame: u64,
    header_bytes: Vec<(Integer, Integer)>,
}

/// Writes a plain dataset and its metadata without replacing existing files.
///
/// The dataset is published before its metadata, so interruption can leave an
/// orphan dataset but not metadata for data that was never published.
pub struct RecordingWriter {
    metadata: Metadata,
    metadata_path: PathBuf,
    dataset_path: PathBuf,
    temporary_dataset_path: PathBuf,
    sample_writer: Option<SampleWriter<File>>,
    dataset_published: bool,
    metadata_published: bool,
}

impl RecordingWriter {
    /// Creates a writer for an unused `.sigmf-meta` path.
    ///
    /// NCD capture headers and trailing bytes are not synthesized.
    pub fn new(metadata_path: impl AsRef<Path>, metadata: Metadata) -> Result<Self, SigmfError> {
        let metadata_path = metadata_path.as_ref().to_path_buf();
        if metadata_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("sigmf-meta")
        {
            return Err(SigmfError::InvalidMetadata(
                "recording metadata path must end with .sigmf-meta".into(),
            ));
        }
        if metadata.global().metadata_only == Some(true) {
            return Err(SigmfError::UnsupportedLayout(
                "metadata-only recordings cannot be created with sample data".into(),
            ));
        }
        if metadata
            .captures()
            .iter()
            .any(|capture| capture.header_bytes.unwrap_or(0) > 0)
            || metadata.global().trailing_bytes.unwrap_or(0) > 0
        {
            return Err(SigmfError::UnsupportedLayout(
                "recording writer does not synthesize NCD headers or trailing bytes".into(),
            ));
        }
        if metadata
            .validate(crate::validation::ValidationPolicy::Interoperability)
            .status
            == crate::validation::ValidationStatus::Failed
        {
            return Err(SigmfError::InvalidMetadata(
                "recording metadata failed validation".into(),
            ));
        }
        let dataset_path = dataset_path_for_metadata(&metadata_path, &metadata)?;
        if dataset_path == metadata_path {
            return Err(SigmfError::InvalidMetadata(
                "dataset and metadata paths must be different".into(),
            ));
        }
        if metadata_path.try_exists()? || dataset_path.try_exists()? {
            return Err(SigmfError::InvalidMetadata(
                "recording writer does not overwrite existing files".into(),
            ));
        }
        let channel_count =
            usize::try_from(metadata.global().num_channels.unwrap_or(1)).map_err(|_| {
                SigmfError::InvalidMetadata("core:num_channels does not fit usize".into())
            })?;
        let (temporary_dataset_path, dataset_file) =
            crate::persistence::create_sibling_temp(&dataset_path)?;
        let sample_writer = match SampleWriter::new(
            dataset_file,
            metadata.global().datatype.clone(),
            channel_count,
        ) {
            Ok(writer) => writer,
            Err(error) => {
                let _ = std::fs::remove_file(&temporary_dataset_path);
                return Err(error);
            }
        };
        Ok(Self {
            metadata,
            metadata_path,
            dataset_path,
            temporary_dataset_path,
            sample_writer: Some(sample_writer),
            dataset_published: false,
            metadata_published: false,
        })
    }

    /// Encodes one complete block of channel frames.
    pub fn write_frames(&mut self, samples: &[SampleValue]) -> Result<(), SigmfError> {
        self.sample_writer
            .as_mut()
            .ok_or_else(|| {
                SigmfError::InvalidMetadata("recording writer is already finalized".into())
            })?
            .write_frames(samples)
    }

    /// Flushes and publishes both files, returning the dataset SHA-512 digest.
    pub fn finish(mut self) -> Result<String, SigmfError> {
        let file = self
            .sample_writer
            .take()
            .ok_or_else(|| {
                SigmfError::InvalidMetadata("recording writer is already finalized".into())
            })?
            .finish()?;
        file.sync_all()?;
        drop(file);
        let mut file = File::open(&self.temporary_dataset_path)?;
        let digest = crate::utils::hash::hash_from_reader(&mut file)?;
        drop(file);

        self.metadata.set_sha512(digest.clone());
        if self
            .metadata
            .validate(crate::validation::ValidationPolicy::Interoperability)
            .status
            == crate::validation::ValidationStatus::Failed
        {
            return Err(SigmfError::InvalidMetadata(
                "recording metadata failed validation after digest update".into(),
            ));
        }
        let metadata_bytes = self.metadata.to_vec()?;
        if metadata_bytes.len() > crate::MAX_META_SIZE {
            return Err(SigmfError::ResourceLimit(format!(
                "metadata exceeds the {} byte limit",
                crate::MAX_META_SIZE
            )));
        }
        let (temporary_metadata_path, mut metadata_file) =
            crate::persistence::create_sibling_temp(&self.metadata_path)?;
        let _metadata_guard = TemporaryFilePath(temporary_metadata_path.clone());
        metadata_file.write_all(&metadata_bytes)?;
        metadata_file.flush()?;
        metadata_file.sync_all()?;
        drop(metadata_file);

        if let Err(error) =
            crate::persistence::publish_new(&self.temporary_dataset_path, &self.dataset_path)
        {
            if crate::utils::hash::hash_from_path(&self.dataset_path)
                .is_ok_and(|published_digest| published_digest == digest)
            {
                self.dataset_published = true;
            }
            return Err(error);
        }
        self.dataset_published = true;
        if let Err(error) =
            crate::persistence::publish_new(&temporary_metadata_path, &self.metadata_path)
        {
            if std::fs::read(&self.metadata_path).is_ok_and(|bytes| bytes == metadata_bytes) {
                self.metadata_published = true;
                return Err(error);
            }
            match std::fs::remove_file(&self.dataset_path) {
                Ok(()) => self.dataset_published = false,
                Err(cleanup_error) => {
                    return Err(SigmfError::InvalidMetadata(format!(
                        "metadata publication failed ({error}) and dataset cleanup failed ({}): {cleanup_error}",
                        self.dataset_path.display()
                    )));
                }
            }
            return Err(error);
        }
        self.metadata_published = true;
        Ok(digest)
    }
}

impl Drop for RecordingWriter {
    fn drop(&mut self) {
        drop(self.sample_writer.take());
        if self.dataset_published && !self.metadata_published {
            let _ = std::fs::remove_file(&self.dataset_path);
        }
        let _ = std::fs::remove_file(&self.temporary_dataset_path);
    }
}

struct TemporaryFilePath(PathBuf);

impl Drop for TemporaryFilePath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn dataset_path_for_metadata(
    metadata_path: &Path,
    metadata: &Metadata,
) -> Result<PathBuf, SigmfError> {
    let filename = metadata_path
        .file_name()
        .and_then(|filename| filename.to_str())
        .ok_or_else(|| {
            SigmfError::InvalidMetadata("metadata filename must be valid UTF-8".into())
        })?;
    let stem = filename.strip_suffix(".sigmf-meta").ok_or_else(|| {
        SigmfError::InvalidMetadata("metadata path must end with .sigmf-meta".into())
    })?;
    let dataset_name = metadata
        .global()
        .dataset
        .clone()
        .unwrap_or_else(|| format!("{stem}.sigmf-data"));
    crate::validate_dataset_filename(&dataset_name)?;
    Ok(metadata_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join(dataset_name))
}

pub struct Recording {
    metadata: Metadata,
    metadata_path: PathBuf,
    dataset_path: Option<PathBuf>,
    dataset: Option<File>,
    dataset_bytes: u64,
}

impl Recording {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SigmfError> {
        Self::open_inner(path.as_ref(), true)
    }

    pub fn open_metadata_only(path: impl AsRef<Path>) -> Result<Self, SigmfError> {
        Self::open_inner(path.as_ref(), false)
    }

    fn open_inner(path: &Path, open_dataset: bool) -> Result<Self, SigmfError> {
        let metadata = Metadata::from_path(path)?;
        let dataset_name = metadata.global().dataset.clone();
        let dataset_path = match dataset_name.as_deref() {
            Some(name) => {
                if name.is_empty()
                    || name.contains('/')
                    || name.contains('\\')
                    || Path::new(name).components().count() != 1
                    || !matches!(
                        Path::new(name).components().next(),
                        Some(std::path::Component::Normal(_))
                    )
                {
                    return Err(SigmfError::InvalidMetadata(
                        "core:dataset must be a filename in the metadata directory".into(),
                    ));
                }
                path.parent().unwrap_or_else(|| Path::new("")).join(name)
            }
            None => {
                let mut dataset_path = path.to_path_buf();
                dataset_path.set_extension("sigmf-data");
                dataset_path
            }
        };
        let metadata_only = metadata.global().metadata_only == Some(true);
        let should_open_dataset =
            open_dataset && (!metadata_only || (dataset_name.is_some() && dataset_path.is_file()));
        let (dataset, actual_dataset_path, dataset_bytes) = if should_open_dataset {
            let file = File::open(&dataset_path)?;
            let dataset_bytes = file.metadata()?.len();
            (Some(file), Some(dataset_path), dataset_bytes)
        } else {
            (None, None, 0)
        };

        Ok(Self {
            metadata,
            metadata_path: path.to_path_buf(),
            dataset_path: actual_dataset_path,
            dataset,
            dataset_bytes,
        })
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn metadata_path(&self) -> &Path {
        &self.metadata_path
    }

    pub fn dataset_path(&self) -> Option<&Path> {
        self.dataset_path.as_deref()
    }

    pub fn first_sample_index(&self) -> Integer {
        self.metadata.global().offset.unwrap_or(0)
    }

    pub fn frame_count(&self) -> Result<Integer, SigmfError> {
        Ok(self.layout()?.frame_count)
    }

    pub fn dataset_digest(&mut self) -> Result<String, SigmfError> {
        let dataset = self.dataset.as_mut().ok_or(SigmfError::NoDataset)?;
        dataset.seek(SeekFrom::Start(0))?;
        crate::utils::hash::hash_from_reader(dataset)
    }

    pub fn verify_dataset_digest(&mut self) -> Result<(), SigmfError> {
        let expected = self
            .metadata
            .global()
            .sha512
            .clone()
            .ok_or_else(|| SigmfError::InvalidMetadata("core:sha512 is not present".into()))?;
        let actual = self.dataset_digest()?;
        if !actual.eq_ignore_ascii_case(&expected) {
            return Err(SigmfError::HashMismatch(format!(
                "dataset digest mismatch: expected {expected}, found {actual}"
            )));
        }
        Ok(())
    }

    fn layout(&self) -> Result<DatasetLayout, SigmfError> {
        if self.dataset.is_none() {
            return Err(SigmfError::NoDataset);
        }
        let global = self.metadata.global();
        let trailer = global.trailing_bytes.unwrap_or(0);
        if trailer > self.dataset_bytes {
            return Err(SigmfError::UnsupportedLayout(
                "core:trailing_bytes exceeds the dataset file size".to_string(),
            ));
        }
        let channels = global.num_channels.unwrap_or(1);
        let channels = usize::try_from(channels).map_err(|_| {
            SigmfError::InvalidMetadata("core:num_channels does not fit usize".to_string())
        })?;
        let bytes_per_frame = u64::try_from(data::bytes_per_frame(&global.datatype, channels)?)
            .map_err(|_| {
                SigmfError::ResourceLimit("frame size does not fit the file address space".into())
            })?;
        let first_sample_index = self.first_sample_index();
        let mut header_bytes: Vec<_> = self
            .metadata
            .captures()
            .iter()
            .filter_map(|capture| {
                let bytes = capture.header_bytes.unwrap_or(0);
                (bytes > 0 && capture.sample_start >= first_sample_index)
                    .then_some((capture.sample_start - first_sample_index, bytes))
            })
            .collect();
        header_bytes.sort_by_key(|(boundary, _)| *boundary);
        let total_header_bytes = header_bytes.iter().try_fold(0u64, |total, (_, bytes)| {
            total.checked_add(*bytes).ok_or_else(|| {
                SigmfError::UnsupportedLayout("capture header size overflows".into())
            })
        })?;
        let sample_bytes = self
            .dataset_bytes
            .checked_sub(trailer)
            .and_then(|bytes| bytes.checked_sub(total_header_bytes))
            .ok_or_else(|| {
                SigmfError::UnsupportedLayout(
                    "capture headers and trailing bytes exceed dataset size".into(),
                )
            })?;
        let frame_count = data::frame_count(sample_bytes, &global.datatype, channels)?;
        if header_bytes
            .iter()
            .any(|(boundary, _)| *boundary > frame_count)
        {
            return Err(SigmfError::UnsupportedLayout(
                "capture header starts beyond the dataset sample range".into(),
            ));
        }
        Ok(DatasetLayout {
            frame_count,
            bytes_per_frame,
            header_bytes,
        })
    }

    /// Reads frames by their absolute SigMF sample indices.
    ///
    /// The stored dataset begins at `core:offset` (or zero when omitted).
    pub fn read_frames(
        &mut self,
        start: Integer,
        count: Integer,
    ) -> Result<Vec<SampleValue>, SigmfError> {
        let layout = self.layout()?;
        let first_sample_index = self.first_sample_index();
        let relative_start = start.checked_sub(first_sample_index).ok_or_else(|| {
            SigmfError::InvalidRange(format!(
                "requested sample index {start} precedes dataset offset {first_sample_index}"
            ))
        })?;
        let relative_end = relative_start.checked_add(count).ok_or_else(|| {
            SigmfError::InvalidRange("requested frame range overflows".to_string())
        })?;
        if relative_end > layout.frame_count {
            return Err(SigmfError::InvalidRange(format!(
                "requested sample range {start}..+{count} exceeds dataset range starting at {first_sample_index} with {} frames",
                layout.frame_count
            )));
        }
        let global = self.metadata.global();
        let channels = usize::try_from(global.num_channels.unwrap_or(1)).map_err(|_| {
            SigmfError::InvalidMetadata("core:num_channels does not fit usize".to_string())
        })?;
        let dataset = self.dataset.as_mut().ok_or(SigmfError::NoDataset)?;
        let sample_count = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(channels))
            .ok_or_else(|| {
                SigmfError::ResourceLimit("requested frame block does not fit memory".into())
            })?;
        let mut samples = Vec::with_capacity(sample_count);
        let mut current = relative_start;
        while current < relative_end {
            let next_boundary = layout
                .header_bytes
                .iter()
                .find_map(|(boundary, _)| (*boundary > current).then_some(*boundary))
                .unwrap_or(relative_end)
                .min(relative_end);
            let chunk_frames = next_boundary - current;
            let samples_before = current
                .checked_mul(layout.bytes_per_frame)
                .ok_or_else(|| SigmfError::InvalidRange("byte offset overflow".into()))?;
            let headers_before = layout
                .header_bytes
                .iter()
                .filter(|(boundary, _)| *boundary <= current)
                .try_fold(0u64, |total, (_, bytes)| total.checked_add(*bytes))
                .ok_or_else(|| SigmfError::InvalidRange("header offset overflow".into()))?;
            let byte_offset = samples_before
                .checked_add(headers_before)
                .ok_or_else(|| SigmfError::InvalidRange("byte offset overflow".into()))?;
            let byte_len = chunk_frames
                .checked_mul(layout.bytes_per_frame)
                .and_then(|length| usize::try_from(length).ok())
                .ok_or_else(|| {
                    SigmfError::ResourceLimit("requested frame block does not fit memory".into())
                })?;
            dataset.seek(SeekFrom::Start(byte_offset))?;
            let mut bytes = vec![0; byte_len];
            dataset.read_exact(&mut bytes)?;
            samples.extend(data::decode(&bytes, &global.datatype, channels)?);
            current = next_boundary;
        }
        Ok(samples)
    }

    pub fn frame_blocks(
        &mut self,
        start: Integer,
        count: Integer,
        block_frames: Integer,
    ) -> Result<FrameBlocks<'_>, SigmfError> {
        if block_frames == 0 {
            return Err(SigmfError::InvalidRange(
                "block size must be greater than zero".to_string(),
            ));
        }
        let frame_count = self.frame_count()?;
        let first_sample_index = self.first_sample_index();
        let relative_start = start.checked_sub(first_sample_index).ok_or_else(|| {
            SigmfError::InvalidRange(format!(
                "requested sample index {start} precedes dataset offset {first_sample_index}"
            ))
        })?;
        let relative_end = relative_start.checked_add(count).ok_or_else(|| {
            SigmfError::InvalidRange("requested frame range overflows".to_string())
        })?;
        if relative_end > frame_count {
            return Err(SigmfError::InvalidRange(format!(
                "requested sample range {start}..+{count} exceeds dataset range starting at {first_sample_index} with {frame_count} frames"
            )));
        }

        Ok(FrameBlocks {
            recording: self,
            first_sample_index,
            next_relative: relative_start,
            remaining: count,
            block_frames,
        })
    }
}

pub struct FrameBlocks<'a> {
    recording: &'a mut Recording,
    first_sample_index: Integer,
    next_relative: Integer,
    remaining: Integer,
    block_frames: Integer,
}

impl Iterator for FrameBlocks<'_> {
    type Item = Result<Vec<SampleValue>, SigmfError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let count = self.remaining.min(self.block_frames);
        let start = match self.first_sample_index.checked_add(self.next_relative) {
            Some(start) => start,
            None => {
                self.remaining = 0;
                return Some(Err(SigmfError::InvalidRange(
                    "absolute sample index overflows".to_string(),
                )));
            }
        };
        match self.recording.read_frames(start, count) {
            Ok(samples) => {
                self.next_relative += count;
                self.remaining -= count;
                Some(Ok(samples))
            }
            Err(error) => {
                self.remaining = 0;
                Some(Err(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::Capture;
    use crate::datatype::Datatype;
    use crate::global::Global;
    use crate::meta::Metadata;
    use std::str::FromStr;

    fn write_recording(root: &Path, datatype: &str, data: &[u8]) -> PathBuf {
        std::fs::create_dir_all(root).unwrap();
        let metadata_path = root.join("capture.sigmf-meta");
        let metadata = Metadata::new(
            Global::new("1.2.6", Datatype::from_str(datatype).unwrap()),
            Vec::new(),
            Vec::new(),
        );
        std::fs::write(&metadata_path, metadata.to_vec().unwrap()).unwrap();
        std::fs::write(root.join("capture.sigmf-data"), data).unwrap();
        metadata_path
    }

    #[test]
    fn reads_requested_frames_relative_to_metadata_path() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-recording-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let metadata_path = write_recording(&root, "ri16_be", &[0, 1, 0, 2, 0, 3]);
        let mut recording = Recording::open(&metadata_path).unwrap();
        assert_eq!(recording.frame_count().unwrap(), 3);
        assert_eq!(
            recording.read_frames(1, 1).unwrap(),
            vec![SampleValue::RealI16(2)]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recording_writer_publishes_dataset_then_matching_metadata() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-recording-writer-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let metadata_path = root.join("written.sigmf-meta");
        let metadata = Metadata::new(
            Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
            Vec::new(),
            Vec::new(),
        );
        let mut writer = RecordingWriter::new(&metadata_path, metadata).unwrap();
        writer
            .write_frames(&[
                SampleValue::RealUnsignedBytes(9),
                SampleValue::RealUnsignedBytes(10),
            ])
            .unwrap();
        let digest = writer.finish().unwrap();

        let metadata = Metadata::from_path(&metadata_path).unwrap();
        assert_eq!(metadata.global().sha512.as_deref(), Some(digest.as_str()));
        let mut recording = Recording::open(&metadata_path).unwrap();
        recording.verify_dataset_digest().unwrap();
        assert_eq!(
            recording.read_frames(0, 2).unwrap(),
            [
                SampleValue::RealUnsignedBytes(9),
                SampleValue::RealUnsignedBytes(10)
            ]
        );
        assert!(RecordingWriter::new(
            &metadata_path,
            Metadata::new(
                Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
                Vec::new(),
                Vec::new(),
            )
        )
        .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recording_writer_rolls_back_dataset_when_metadata_publication_fails() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-recording-rollback-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let metadata_path = root.join("written.sigmf-meta");
        let dataset_path = root.join("written.sigmf-data");
        let metadata = Metadata::new(
            Global::new("1.2.6", Datatype::from_str("ru8").unwrap()),
            Vec::new(),
            Vec::new(),
        );
        let mut writer = RecordingWriter::new(&metadata_path, metadata).unwrap();
        writer
            .write_frames(&[SampleValue::RealUnsignedBytes(9)])
            .unwrap();
        std::fs::write(&metadata_path, b"existing metadata").unwrap();

        assert!(writer.finish().is_err());
        assert_eq!(std::fs::read(&metadata_path).unwrap(), b"existing metadata");
        assert!(!dataset_path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maps_absolute_sample_indices_through_core_offset() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-offset-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let metadata_path = write_recording(&root, "ru8", &[10, 20, 30]);
        let mut metadata = Metadata::from_path(&metadata_path).unwrap();
        metadata.set_offset(1000);
        std::fs::write(&metadata_path, metadata.to_vec().unwrap()).unwrap();

        let mut recording = Recording::open(&metadata_path).unwrap();
        assert_eq!(recording.first_sample_index(), 1000);
        assert_eq!(recording.frame_count().unwrap(), 3);
        assert_eq!(
            recording.read_frames(1001, 1).unwrap(),
            vec![SampleValue::RealUnsignedBytes(20)]
        );
        assert!(matches!(
            recording.read_frames(999, 1),
            Err(SigmfError::InvalidRange(_))
        ));
        assert!(matches!(
            recording.read_frames(1003, 1),
            Err(SigmfError::InvalidRange(_))
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn frame_blocks_are_bounded_and_keep_errors_visible() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-frame-blocks-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let metadata_path = write_recording(&root, "ru8", &[1, 2, 3, 4, 5]);
        let mut recording = Recording::open(&metadata_path).unwrap();
        let blocks: Vec<_> = recording.frame_blocks(0, 5, 2).unwrap().collect();
        assert_eq!(
            blocks
                .into_iter()
                .map(Result::unwrap)
                .map(|block| block.len())
                .collect::<Vec<_>>(),
            [2, 2, 1]
        );
        assert!(recording.frame_blocks(0, 1, 0).is_err());
        assert!(recording.frame_blocks(4, 2, 1).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_samples_across_capture_headers() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-capture-headers-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let metadata_path = root.join("header.sigmf-meta");
        let mut global = Global::new("1.2.6", Datatype::from_str("ru8").unwrap());
        global.set_offset(100);
        let mut first_capture = Capture::new(100);
        first_capture.set_header_bytes(1);
        let mut second_capture = Capture::new(102);
        second_capture.set_header_bytes(1);
        let metadata = Metadata::new(global, Vec::new(), vec![first_capture, second_capture]);
        std::fs::write(&metadata_path, metadata.to_vec().unwrap()).unwrap();
        std::fs::write(root.join("header.sigmf-data"), [0xaa, 1, 2, 0xbb, 3]).unwrap();

        let mut recording = Recording::open(&metadata_path).unwrap();
        assert_eq!(recording.frame_count().unwrap(), 3);
        assert_eq!(
            recording.read_frames(100, 3).unwrap(),
            [
                SampleValue::RealUnsignedBytes(1),
                SampleValue::RealUnsignedBytes(2),
                SampleValue::RealUnsignedBytes(3)
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn trailer_bytes_are_excluded_from_frames_but_included_in_digest() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-trailer-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let metadata_path = write_recording(&root, "ru8", &[1, 2, 3, 0xaa, 0xbb]);
        let mut metadata = Metadata::from_path(&metadata_path).unwrap();
        metadata.set_trailing_bytes(2);
        std::fs::write(&metadata_path, metadata.to_vec().unwrap()).unwrap();

        let mut recording = Recording::open(&metadata_path).unwrap();
        assert_eq!(recording.frame_count().unwrap(), 3);
        assert_eq!(
            recording.read_frames(0, 3).unwrap(),
            [
                SampleValue::RealUnsignedBytes(1),
                SampleValue::RealUnsignedBytes(2),
                SampleValue::RealUnsignedBytes(3)
            ]
        );
        assert_eq!(
            recording.dataset_digest().unwrap(),
            crate::utils::hash::hash_from_slice(&[1, 2, 3, 0xaa, 0xbb]).unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn computes_and_verifies_hash_over_exact_dataset_bytes() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-dataset-hash-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let metadata_path = write_recording(&root, "ru8", &[1, 2, 3]);
        let mut metadata = Metadata::from_path(&metadata_path).unwrap();
        let digest = crate::utils::hash::hash_from_slice(&[1, 2, 3]).unwrap();
        metadata.set_sha512(digest.clone());
        std::fs::write(&metadata_path, metadata.to_vec().unwrap()).unwrap();

        let mut recording = Recording::open(&metadata_path).unwrap();
        assert_eq!(recording.dataset_digest().unwrap(), digest);
        recording.verify_dataset_digest().unwrap();
        metadata.set_sha512("0".repeat(128));
        std::fs::write(&metadata_path, metadata.to_vec().unwrap()).unwrap();
        let mut recording = Recording::open(&metadata_path).unwrap();
        assert!(matches!(
            recording.verify_dataset_digest(),
            Err(SigmfError::HashMismatch(_))
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_only_recording_reports_unavailable_samples() {
        let root = std::env::temp_dir().join(format!(
            "sigmf-metadata-only-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("meta.sigmf-meta");
        let mut global = Global::new("1.2.6", Datatype::from_str("ru8").unwrap());
        global.set_metadata_only(true);
        std::fs::write(
            &path,
            Metadata::new(global, Vec::new(), Vec::new())
                .to_vec()
                .unwrap(),
        )
        .unwrap();
        let mut recording = Recording::open(&path).unwrap();
        assert!(matches!(
            recording.frame_count(),
            Err(SigmfError::NoDataset)
        ));
        assert!(matches!(
            recording.read_frames(0, 1),
            Err(SigmfError::NoDataset)
        ));
        std::fs::remove_dir_all(root).unwrap();
    }
}
