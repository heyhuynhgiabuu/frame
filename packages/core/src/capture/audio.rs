//! Audio capture implementation using cpal for microphone input
//!
//! This module provides:
//! - Microphone capture via cpal (cross-platform)
//! - Audio mixing for combining multiple sources
//! - Sample rate conversion via rubato

#[cfg(feature = "capture")]
pub mod microphone {
    use crate::capture::AudioBuffer;
    use crate::{FrameError, FrameResult};
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Audio capture configuration
    #[derive(Debug, Clone)]
    pub struct AudioConfig {
        /// Sample rate (e.g., 48000)
        pub sample_rate: u32,
        /// Number of channels (1 = mono, 2 = stereo)
        pub channels: u16,
        /// Buffer size in samples
        pub buffer_size: u32,
    }

    impl Default for AudioConfig {
        fn default() -> Self {
            Self {
                sample_rate: 48000,
                channels: 2,
                buffer_size: 1024,
            }
        }
    }

    /// Information about an audio input device
    #[derive(Debug, Clone)]
    pub struct AudioDeviceInfo {
        /// Device name
        pub name: String,
        /// Whether this is the default device
        pub is_default: bool,
        /// Supported sample rates
        pub supported_sample_rates: Vec<u32>,
        /// Maximum channels
        pub max_channels: u16,
    }

    /// Microphone capture using cpal
    pub struct MicrophoneCapture {
        is_recording: Arc<AtomicBool>,
        audio_rx: Option<mpsc::Receiver<AudioBuffer>>,
        config: AudioConfig,
        stream: Option<cpal::Stream>,
        start_time: Option<std::time::Instant>,
    }

    impl MicrophoneCapture {
        /// Create a new microphone capture instance
        pub fn new(config: AudioConfig) -> FrameResult<Self> {
            Ok(Self {
                is_recording: Arc::new(AtomicBool::new(false)),
                audio_rx: None,
                config,
                stream: None,
                start_time: None,
            })
        }

        /// List available audio input devices
        pub fn list_devices() -> FrameResult<Vec<AudioDeviceInfo>> {
            let host = cpal::default_host();
            let default_device = host.default_input_device();
            let default_name = default_device.as_ref().and_then(|d| d.name().ok());

            let devices = host
                .input_devices()
                .map_err(|e| FrameError::AudioError(format!("Failed to list devices: {}", e)))?;

            let mut result = Vec::new();

            for device in devices {
                let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                let is_default = default_name.as_ref().map(|n| n == &name).unwrap_or(false);

                // Get supported configs
                let supported_configs = device.supported_input_configs();
                let (sample_rates, max_channels) = match supported_configs {
                    Ok(configs) => {
                        let configs: Vec<_> = configs.collect();
                        let sample_rates: Vec<u32> = configs
                            .iter()
                            .flat_map(|c| vec![c.min_sample_rate().0, c.max_sample_rate().0])
                            .collect();
                        let max_channels = configs.iter().map(|c| c.channels()).max().unwrap_or(2);
                        (sample_rates, max_channels)
                    }
                    Err(_) => (vec![48000], 2),
                };

                result.push(AudioDeviceInfo {
                    name,
                    is_default,
                    supported_sample_rates: sample_rates,
                    max_channels,
                });
            }

            Ok(result)
        }

        /// Get the default input device
        pub fn default_device() -> FrameResult<AudioDeviceInfo> {
            let host = cpal::default_host();
            let device = host.default_input_device().ok_or_else(|| {
                FrameError::AudioError("No default input device found".to_string())
            })?;

            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());

            let supported_configs = device
                .supported_input_configs()
                .map_err(|e| FrameError::AudioError(format!("Failed to get configs: {}", e)))?;

            let configs: Vec<_> = supported_configs.collect();
            let sample_rates: Vec<u32> = configs
                .iter()
                .flat_map(|c| vec![c.min_sample_rate().0, c.max_sample_rate().0])
                .collect();
            let max_channels = configs.iter().map(|c| c.channels()).max().unwrap_or(2);

            Ok(AudioDeviceInfo {
                name,
                is_default: true,
                supported_sample_rates: sample_rates,
                max_channels,
            })
        }

        /// Start capturing audio from the microphone
        pub fn start(&mut self) -> FrameResult<()> {
            if self.is_recording.load(Ordering::SeqCst) {
                return Err(FrameError::RecordingInProgress);
            }

            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or_else(|| FrameError::AudioError("No default input device".to_string()))?;

            // Configure audio stream
            let stream_config = cpal::StreamConfig {
                channels: self.config.channels,
                sample_rate: cpal::SampleRate(self.config.sample_rate),
                buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size),
            };

            // Create channel for audio data
            let (audio_tx, audio_rx) = mpsc::channel::<AudioBuffer>(100);
            self.audio_rx = Some(audio_rx);

            let is_recording = self.is_recording.clone();
            let sample_rate = self.config.sample_rate;
            let channels = self.config.channels;
            let start_time = std::time::Instant::now();
            self.start_time = Some(start_time);

            // Error callback
            let err_fn = |err| {
                tracing::error!("Audio stream error: {}", err);
            };

            // Build the input stream
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !is_recording.load(Ordering::SeqCst) {
                            return;
                        }

                        let timestamp = start_time.elapsed();
                        let buffer = AudioBuffer {
                            samples: data.to_vec(),
                            sample_rate,
                            channels,
                            timestamp,
                        };

                        // Non-blocking send
                        let _ = audio_tx.try_send(buffer);
                    },
                    err_fn,
                    None, // timeout
                )
                .map_err(|e| FrameError::AudioError(format!("Failed to build stream: {}", e)))?;

            // Start the stream
            stream
                .play()
                .map_err(|e| FrameError::AudioError(format!("Failed to start stream: {}", e)))?;

            self.stream = Some(stream);
            self.is_recording.store(true, Ordering::SeqCst);

            tracing::info!(
                "Microphone capture started: {}Hz, {} channels",
                self.config.sample_rate,
                self.config.channels
            );

            Ok(())
        }

        /// Stop capturing audio
        pub fn stop(&mut self) -> FrameResult<()> {
            if !self.is_recording.load(Ordering::SeqCst) {
                return Err(FrameError::NoRecordingInProgress);
            }

            self.is_recording.store(false, Ordering::SeqCst);

            // Drop the stream to stop it
            self.stream = None;
            self.start_time = None;

            tracing::info!("Microphone capture stopped");

            Ok(())
        }

        /// Get the next audio buffer (non-blocking)
        pub async fn next_buffer(&mut self) -> FrameResult<Option<AudioBuffer>> {
            if let Some(ref mut rx) = self.audio_rx {
                match rx.try_recv() {
                    Ok(buffer) => Ok(Some(buffer)),
                    Err(mpsc::error::TryRecvError::Empty) => Ok(None),
                    Err(mpsc::error::TryRecvError::Disconnected) => Err(FrameError::AudioError(
                        "Audio channel disconnected".to_string(),
                    )),
                }
            } else {
                Ok(None)
            }
        }

        /// Check if currently recording
        pub fn is_recording(&self) -> bool {
            self.is_recording.load(Ordering::SeqCst)
        }
    }

    impl Drop for MicrophoneCapture {
        fn drop(&mut self) {
            if self.is_recording.load(Ordering::SeqCst) {
                let _ = self.stop();
            }
        }
    }
}

/// Audio mixer for combining multiple audio sources
#[cfg(feature = "capture")]
pub mod mixer {
    use crate::capture::AudioBuffer;
    use crate::{FrameError, FrameResult};

    /// Mixes multiple audio buffers together
    pub struct AudioMixer {
        #[allow(dead_code)]
        sample_rate: u32,
        #[allow(dead_code)]
        channels: u16,
    }

    impl AudioMixer {
        /// Create a new audio mixer
        pub fn new(sample_rate: u32, channels: u16) -> Self {
            Self {
                sample_rate,
                channels,
            }
        }

        /// Mix two audio buffers together
        ///
        /// Both buffers must have the same sample rate and channel count
        pub fn mix(&self, a: &AudioBuffer, b: &AudioBuffer) -> FrameResult<AudioBuffer> {
            if a.sample_rate != b.sample_rate {
                return Err(FrameError::AudioError(format!(
                    "Sample rate mismatch: {} vs {}",
                    a.sample_rate, b.sample_rate
                )));
            }

            if a.channels != b.channels {
                return Err(FrameError::AudioError(format!(
                    "Channel count mismatch: {} vs {}",
                    a.channels, b.channels
                )));
            }

            let len = a.samples.len().max(b.samples.len());
            let mut mixed = vec![0.0f32; len];

            // Mix with simple averaging (prevents clipping)
            for (i, sample) in mixed.iter_mut().enumerate() {
                let sample_a = a.samples.get(i).copied().unwrap_or(0.0);
                let sample_b = b.samples.get(i).copied().unwrap_or(0.0);
                *sample = (sample_a + sample_b) * 0.5;
            }

            // Use the earlier timestamp
            let timestamp = a.timestamp.min(b.timestamp);

            Ok(AudioBuffer {
                samples: mixed,
                sample_rate: a.sample_rate,
                channels: a.channels,
                timestamp,
            })
        }

        /// Mix multiple audio buffers together
        pub fn mix_all(&self, buffers: &[AudioBuffer]) -> FrameResult<Option<AudioBuffer>> {
            if buffers.is_empty() {
                return Ok(None);
            }

            if buffers.len() == 1 {
                return Ok(Some(buffers[0].clone()));
            }

            let mut result = buffers[0].clone();
            for buffer in &buffers[1..] {
                result = self.mix(&result, buffer)?;
            }

            Ok(Some(result))
        }
    }
}

/// Sample rate converter using rubato
#[cfg(feature = "capture")]
pub mod resampler {
    use crate::capture::AudioBuffer;
    use crate::{FrameError, FrameResult};
    use rubato::{FftFixedIn, Resampler as RubatoResampler};

    /// Converts audio between different sample rates
    pub struct SampleRateConverter {
        from_rate: u32,
        to_rate: u32,
        channels: u16,
    }

    impl SampleRateConverter {
        /// Create a new sample rate converter
        pub fn new(from_rate: u32, to_rate: u32, channels: u16) -> Self {
            Self {
                from_rate,
                to_rate,
                channels,
            }
        }

        /// Convert an audio buffer to the target sample rate
        pub fn convert(&self, buffer: &AudioBuffer) -> FrameResult<AudioBuffer> {
            if self.from_rate == self.to_rate {
                return Ok(buffer.clone());
            }

            // Create resampler
            let mut resampler = FftFixedIn::<f64>::new(
                self.from_rate as usize,
                self.to_rate as usize,
                buffer.samples.len() / self.channels as usize,
                1, // sub_chunks
                self.channels as usize,
            )
            .map_err(|e| FrameError::AudioError(format!("Failed to create resampler: {}", e)))?;

            // Deinterleave channels
            let num_frames = buffer.samples.len() / self.channels as usize;
            let channels_data: Vec<Vec<f64>> = (0..self.channels)
                .map(|ch| {
                    (0..num_frames)
                        .map(|frame| {
                            buffer.samples[frame * self.channels as usize + ch as usize] as f64
                        })
                        .collect()
                })
                .collect();

            // Process
            let output = resampler
                .process(&channels_data, None)
                .map_err(|e| FrameError::AudioError(format!("Resampling failed: {}", e)))?;

            // Interleave output
            let output_frames = output[0].len();
            let mut interleaved = Vec::with_capacity(output_frames * self.channels as usize);

            for frame in 0..output_frames {
                for ch in 0..self.channels as usize {
                    interleaved.push(output[ch][frame] as f32);
                }
            }

            Ok(AudioBuffer {
                samples: interleaved,
                sample_rate: self.to_rate,
                channels: self.channels,
                timestamp: buffer.timestamp,
            })
        }
    }
}

#[cfg(all(test, feature = "capture"))]
mod tests {
    use super::*;
    use crate::capture::AudioBuffer;

    #[test]
    fn test_audio_mixer() {
        let mixer = mixer::AudioMixer::new(48000, 2);

        let a = AudioBuffer {
            samples: vec![0.5, 0.5, 0.3, 0.3],
            sample_rate: 48000,
            channels: 2,
            timestamp: std::time::Duration::from_secs(0),
        };

        let b = AudioBuffer {
            samples: vec![0.3, 0.3, 0.5, 0.5],
            sample_rate: 48000,
            channels: 2,
            timestamp: std::time::Duration::from_secs(0),
        };

        let mixed = mixer.mix(&a, &b).unwrap();
        assert_eq!(mixed.samples.len(), 4);
        // Should be averaged: (0.5 + 0.3) / 2 = 0.4
        assert!((mixed.samples[0] - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_sample_rate_converter() {
        // Just test creation - actual conversion requires real audio data
        let _converter = resampler::SampleRateConverter::new(44100, 48000, 2);
    }
}
