use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    HeapCons, HeapProd, HeapRb,
    traits::{Consumer, Producer, Split},
};

pub const SAMPLE_RATE: u32 = 44_100;
pub const CHANNELS: u16 = 2;

#[derive(Clone)]
pub struct AudioWriter {
    producer: Arc<std::sync::Mutex<HeapProd<f32>>>,
    clear_requested: Arc<AtomicBool>,
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

        if supported.sample_format() != cpal::SampleFormat::F32
            || supported.sample_rate() != SAMPLE_RATE
            || supported.channels() != CHANNELS
        {
            bail!("audio device does not support 44.1 kHz stereo f32 playback");
        }

        let config: cpal::StreamConfig = supported.into();
        let rb = HeapRb::<f32>::new(SAMPLE_RATE as usize * CHANNELS as usize);
        let (producer, consumer) = rb.split();
        let producer = Arc::new(std::sync::Mutex::new(producer));
        let clear_requested = Arc::new(AtomicBool::new(false));
        let writer = AudioWriter {
            producer,
            clear_requested: Arc::clone(&clear_requested),
        };
        let stream = build_stream(&device, &config, consumer, clear_requested)?;
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
    mut consumer: HeapCons<f32>,
    clear_requested: Arc<AtomicBool>,
) -> Result<cpal::Stream> {
    let err_fn = |err| tracing::error!(error = %err, "audio stream error");
    let stream = device.build_output_stream(
        config,
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if clear_requested.swap(false, Ordering::AcqRel) {
                while consumer.try_pop().is_some() {}
            }
            for sample in output.iter_mut() {
                *sample = consumer.try_pop().unwrap_or(0.0);
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}
