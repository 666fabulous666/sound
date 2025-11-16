#[cfg(not(target_arch = "wasm32"))]
mod native {
    use anyhow::{Context, Result};
    use hound::{SampleFormat, WavSpec, WavWriter};
    use std::{
        fs::File,
        io::BufWriter,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    struct RecorderInner {
        writer: WavWriter<BufWriter<File>>,
        path: PathBuf,
    }

    impl RecorderInner {
        fn new(path: PathBuf, sample_rate: u32) -> Result<Self> {
            let spec = WavSpec {
                channels: 2,
                sample_rate,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            };
            let writer = WavWriter::create(&path, spec)
                .with_context(|| format!("failed to create WAV file {}", path.display()))?;
            Ok(Self { writer, path })
        }
    }

    /// Thread-safe recorder that can capture stereo frames from the audio callback.
    pub struct Recorder {
        inner: Mutex<Option<RecorderInner>>,
    }

    impl Recorder {
        pub fn new() -> Self {
            Self {
                inner: Mutex::new(None),
            }
        }

        /// Start recording to the given path with the provided sample rate.
        pub fn start<P: AsRef<Path>>(&self, path: P, sample_rate: u32) -> Result<()> {
            let path = path.as_ref().to_path_buf();
            let mut guard = self.inner.lock().expect("recorder mutex poisoned");
            if guard.is_some() {
                anyhow::bail!("Recorder already active");
            }
            *guard = Some(RecorderInner::new(path, sample_rate)?);
            Ok(())
        }

        /// Stop recording and finalize the WAV file.
        pub fn stop(&self) -> Result<Option<PathBuf>> {
            let mut guard = self.inner.lock().expect("recorder mutex poisoned");
            if let Some(inner) = guard.take() {
                let path = inner.path.clone();
                inner.writer.finalize().context("Failed to finalize WAV")?;
                Ok(Some(path))
            } else {
                Ok(None)
            }
        }

        pub fn is_recording(&self) -> bool {
            self.inner
                .lock()
                .map(|guard| guard.is_some())
                .unwrap_or(false)
        }

        /// Write a stereo frame (left/right) into the WAV file if recording.
        pub fn write_frame(&self, left: f32, right: f32) {
            if let Ok(mut guard) = self.inner.lock() {
                if let Some(inner) = guard.as_mut() {
                    let _ = inner.writer.write_sample(left);
                    let _ = inner.writer.write_sample(right);
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod native {
    use anyhow::Result;
    use std::path::{Path, PathBuf};

    #[derive(Clone)]
    pub struct Recorder;

    impl Recorder {
        pub fn new() -> Self {
            Recorder
        }

        pub fn start<P: AsRef<Path>>(&self, _path: P, _sample_rate: u32) -> Result<()> {
            Ok(())
        }

        pub fn stop(&self) -> Result<Option<PathBuf>> {
            Ok(None)
        }

        pub fn is_recording(&self) -> bool {
            false
        }

        pub fn write_frame(&self, _left: f32, _right: f32) {}
    }
}

pub use native::Recorder;
