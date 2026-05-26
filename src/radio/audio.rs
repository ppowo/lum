use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use cpal::{
    FromSample, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use ringbuf::{
    HeapCons, HeapProd, HeapRb,
    traits::{Consumer, Producer, Split},
};

#[cfg(test)]
pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;

#[derive(Clone)]
pub struct AudioWriter {
    producer: Arc<std::sync::Mutex<HeapProd<f32>>>,
    clear_requested: Arc<AtomicBool>,
    sample_rate: u32,
}

impl AudioWriter {
    pub fn push_samples(&self, samples: &[f32], stop: &AtomicBool) {
        let mut offset = 0;
        while offset < samples.len() && !stop.load(Ordering::Relaxed) {
            let pushed = {
                let mut producer = self.producer.lock().expect("audio producer poisoned");
                producer.push_slice(&samples[offset..])
            };
            offset += pushed;
            if pushed == 0 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    pub fn clear(&self) {
        self.clear_requested.store(true, Ordering::Release);
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

pub struct AudioPlayer {
    _stream: cpal::Stream,
    writer: AudioWriter,
}

impl AudioPlayer {
    pub fn open() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("audio unavailable: no default output device")?;
        let supported = device
            .default_output_config()
            .context("audio unavailable: no default output config")?;
        tracing::info!(config = ?supported, "default audio output config");

        let config: cpal::StreamConfig = supported.clone().into();
        let playback = playback_config(&supported);
        let rb = HeapRb::<f32>::new(playback.sample_rate as usize * playback.channels as usize);
        let (producer, consumer) = rb.split();
        let producer = Arc::new(std::sync::Mutex::new(producer));
        let clear_requested = Arc::new(AtomicBool::new(false));
        let writer = AudioWriter {
            producer,
            clear_requested: Arc::clone(&clear_requested),
            sample_rate: playback.sample_rate,
        };
        let stream = build_stream(
            &device,
            &config,
            supported.sample_format(),
            consumer,
            clear_requested,
        )?;
        stream
            .play()
            .context("audio unavailable: failed to start output stream")?;

        Ok(Self {
            _stream: stream,
            writer,
        })
    }

    pub fn writer(&self) -> AudioWriter {
        self.writer.clone()
    }

    pub fn clear(&self) {
        self.writer.clear();
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    consumer: HeapCons<f32>,
    clear_requested: Arc<AtomicBool>,
) -> Result<cpal::Stream> {
    match sample_format {
        cpal::SampleFormat::F32 => {
            build_stream_for::<f32>(device, config, consumer, clear_requested)
        }
        cpal::SampleFormat::I16 => {
            build_stream_for::<i16>(device, config, consumer, clear_requested)
        }
        cpal::SampleFormat::U16 => {
            build_stream_for::<u16>(device, config, consumer, clear_requested)
        }
        format => anyhow::bail!("audio device uses unsupported sample format: {format:?}"),
    }
}

fn build_stream_for<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: HeapCons<f32>,
    clear_requested: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            if clear_requested.swap(false, Ordering::AcqRel) {
                while consumer.try_pop().is_some() {}
            }
            for sample in output.iter_mut() {
                *sample = output_sample(consumer.try_pop().unwrap_or(0.0));
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn err_fn(error: cpal::StreamError) {
    eprintln!("audio stream error: {error}");
}


fn output_sample<T>(sample: f32) -> T
where
    T: SizedSample + FromSample<f32>,
{
    T::from_sample(sample)
}

struct PlaybackConfig {
    sample_rate: u32,
    channels: u16,
}

fn playback_config(config: &cpal::SupportedStreamConfig) -> PlaybackConfig {
    PlaybackConfig {
        sample_rate: config.sample_rate(),
        channels: config.channels(),
    }
}

#[cfg(test)]
mod tests {
    use cpal::{SampleFormat, SupportedBufferSize, SupportedStreamConfigRange};

    #[test]
    fn accepts_default_output_config_that_is_not_44_1khz_f32() {
        let supported = SupportedStreamConfigRange::new(
            2,
            48_000,
            48_000,
            SupportedBufferSize::Unknown,
            SampleFormat::I16,
        )
        .with_sample_rate(48_000);

        let playback = super::playback_config(&supported);

        assert_eq!(playback.sample_rate, 48_000);
        assert_eq!(playback.channels, 2);
    }


    #[test]
    fn f32_output_preserves_sample_amplitude() {
        assert_eq!(super::output_sample::<f32>(0.0), 0.0);
        assert_eq!(super::output_sample::<f32>(0.5), 0.5);
        assert_eq!(super::output_sample::<f32>(-0.5), -0.5);
    }

    #[test]
    fn integer_output_preserves_silence_and_signal_direction() {
        assert_eq!(super::output_sample::<i16>(0.0), 0);
        assert_eq!(super::output_sample::<u16>(0.0), 32_768);

        assert!(super::output_sample::<i16>(0.5) > 0);
        assert!(super::output_sample::<i16>(-0.5) < 0);
        assert!(super::output_sample::<u16>(0.5) > 32_768);
        assert!(super::output_sample::<u16>(-0.5) < 32_768);
    }
}
