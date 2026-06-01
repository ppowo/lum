use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Context, Result, bail};
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};
use std::time::Duration;
use symphonia::core::{
    audio::sample::Sample,
    codecs::audio::AudioDecoderOptions,
    errors::Error as SymphoniaError,
    formats::{FormatOptions, FormatReader, TrackType, probe::Hint},
    io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
};

use super::{
    audio::AudioWriter,
    stations::{Station, StationKind},
};
use crate::ffmpeg;

struct HttpStream {
    inner: Mutex<reqwest::blocking::Response>,
}

impl HttpStream {
    fn new(response: reqwest::blocking::Response) -> Self {
        Self {
            inner: Mutex::new(response),
        }
    }
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.lock().expect("http stream poisoned").read(buf)
    }
}

impl Seek for HttpStream {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::other("http stream does not support seeking"))
    }
}

impl MediaSource for HttpStream {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

pub fn resolve_station_url(station: &Station, yt_dlp: Option<&Path>) -> Result<String> {
    match station.kind {
        StationKind::Direct => Ok(station.url.to_string()),
        StationKind::YouTube => {
            let yt_dlp = yt_dlp
                .ok_or_else(|| anyhow::anyhow!("yt-dlp binary required for YouTube station"))?;
            let output = Command::new(yt_dlp)
                .args(["-g", "--no-playlist", station.url])
                .output()
                .context("failed to run yt-dlp")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("yt-dlp failed: {stderr}");
            }
            let url = String::from_utf8(output.stdout)
                .context("yt-dlp output is not valid UTF-8")?
                .trim()
                .to_string();
            Ok(url)
        }
    }
}

pub fn stream_station(
    station: Station,
    writer: AudioWriter,
    stop: Arc<AtomicBool>,
    yt_dlp: Option<&Path>,
    ffmpeg: Option<&Path>,
) -> Result<()> {
    tracing::info!(
        station = station.code,
        url = station.url,
        "opening station stream"
    );

    match station.kind {
        StationKind::Direct => stream_direct_station(station, writer, stop),
        StationKind::YouTube => {
            stream_youtube_station_via_ffmpeg(station, writer, stop, yt_dlp, ffmpeg)
        }
    }
}

fn stream_direct_station(
    station: Station,
    writer: AudioWriter,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    const MAX_RECONNECTS: u32 = 5;

    decode_with_retry(
        &mut || try_decode_session(station.url, station.code, writer.clone(), &stop),
        &mut || Ok(()),
        is_reconnectable_err,
        MAX_RECONNECTS,
        &stop,
    )
}

fn stream_youtube_station_via_ffmpeg(
    station: Station,
    writer: AudioWriter,
    stop: Arc<AtomicBool>,
    yt_dlp: Option<&Path>,
    ffmpeg: Option<&Path>,
) -> Result<()> {
    let yt_dlp =
        yt_dlp.ok_or_else(|| anyhow::anyhow!("yt-dlp binary required for YouTube station"))?;
    let ffmpeg =
        ffmpeg.ok_or_else(|| anyhow::anyhow!("ffmpeg binary required for YouTube station"))?;
    let url = resolve_station_url(&station, Some(yt_dlp))?;
    stream_ffmpeg_pcm(ffmpeg, &url, writer, &stop)
}

fn stream_ffmpeg_pcm(
    ffmpeg: &Path,
    url: &str,
    writer: AudioWriter,
    stop: &AtomicBool,
) -> Result<()> {
    let mut pcm = ffmpeg::PcmStdout::spawn(ffmpeg, url, writer.sample_rate())?;
    let mut bytes = [0_u8; 16 * 1024];
    let mut carry = Vec::<u8>::new();

    while !stop.load(Ordering::Relaxed) {
        let read = pcm
            .read(&mut bytes)
            .context("failed to read ffmpeg PCM output")?;
        if read == 0 {
            break;
        }
        carry.extend_from_slice(&bytes[..read]);
        let sample_bytes = carry.len() / 4 * 4;
        if sample_bytes == 0 {
            continue;
        }
        let samples: Vec<f32> = carry[..sample_bytes]
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        carry.drain(..sample_bytes);
        writer.push_samples(&samples, stop);
    }

    if stop.load(Ordering::Relaxed) {
        pcm.kill();
        return Ok(());
    }

    pcm.finish()
}

fn try_decode_session(url: &str, code: &str, writer: AudioWriter, stop: &AtomicBool) -> Result<()> {
    let mut format = open_station_format(url, code)?;
    let track = format
        .default_track(TrackType::Audio)
        .context("stream has no audio track")?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .context("audio track has no codec parameters")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .context("failed to create audio decoder")?;
    let mut samples = Vec::<f32>::new();
    let mut normalizer = AudioNormalizer::new(writer.sample_rate());

    while !stop.load(Ordering::Relaxed) {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::Interrupted =>
            {
                continue;
            }
            Err(error) => return Err(error).context("failed to read audio packet"),
        };
        if packet.track_id != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(buffer) => {
                samples.resize(buffer.samples_interleaved(), f32::MID);
                buffer.copy_to_slice_interleaved(&mut samples);
                let output = normalizer.normalize(buffer.spec(), &samples)?;
                writer.push_samples(&output, stop);
            }
            Err(SymphoniaError::DecodeError(error)) => {
                tracing::warn!(error = %error, "skipping undecodable packet");
            }
            Err(error) => return Err(error).context("failed to decode audio packet"),
        }
    }

    tracing::info!(station = code, "station stream stopped");
    Ok(())
}

#[cfg(test)]
pub(crate) fn probe_station_decode(station: Station, packets_to_decode: usize) -> Result<usize> {
    let mut format = open_station_format(station.url, station.code)?;
    let track = format
        .default_track(TrackType::Audio)
        .context("stream has no audio track")?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .context("audio track has no codec parameters")?;
    // Some live streams do not expose full channel metadata until packets are decoded.
    // Validate decoded buffers below instead of rejecting incomplete probe metadata.

    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .context("failed to create audio decoder")?;
    let mut decoded = 0;

    let mut normalizer = AudioNormalizer::new(super::audio::DEFAULT_SAMPLE_RATE);

    while decoded < packets_to_decode {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => return Err(error).context("failed to read audio packet"),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buffer) => {
                let mut samples = vec![f32::MID; buffer.samples_interleaved()];
                buffer.copy_to_slice_interleaved(&mut samples);
                let output = normalizer.normalize(buffer.spec(), &samples)?;
                if output.is_empty() {
                    bail!("station decoded empty audio packet");
                }
                decoded += 1;
            }
            Err(SymphoniaError::DecodeError(error)) => {
                tracing::warn!(error = %error, "skipping undecodable packet")
            }
            Err(error) => return Err(error).context("failed to decode audio packet"),
        }
    }

    if decoded == 0 {
        bail!("station did not decode any audio packets");
    }
    Ok(decoded)
}

#[cfg(test)]
pub(crate) fn probe_youtube_station_via_ffmpeg(
    station: &Station,
    yt_dlp: &Path,
    ffmpeg: &Path,
    min_samples: usize,
) -> Result<usize> {
    let url = resolve_station_url(station, Some(yt_dlp))?;
    let mut pcm = ffmpeg::PcmStdout::spawn(ffmpeg, &url, super::audio::DEFAULT_SAMPLE_RATE)?;
    let mut bytes = vec![0; min_samples * std::mem::size_of::<f32>()];
    let mut read = 0;
    while read < bytes.len() {
        let n = pcm
            .read(&mut bytes[read..])
            .context("failed to read ffmpeg PCM output")?;
        if n == 0 {
            break;
        }
        read += n;
    }
    pcm.kill();

    let samples = read / std::mem::size_of::<f32>();
    if samples == 0 {
        bail!("ffmpeg did not produce PCM samples");
    }
    Ok(samples)
}

fn open_station_format(url: &str, code: &str) -> Result<Box<dyn FormatReader>> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .context("failed to build HTTP client")?;
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "lum/0.1")
        .send()
        .with_context(|| format!("failed to open stream for {code}"))?
        .error_for_status()
        .with_context(|| format!("stream returned error status for {code}"))?;

    let mss = MediaSourceStream::new(
        Box::new(HttpStream::new(response)),
        MediaSourceStreamOptions {
            ..Default::default()
        },
    );
    let hint = Hint::new();
    symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("failed to detect stream format")
}
/// Returns `true` if the error is a reconnectable I/O error from the underlying HTTP stream.
pub(crate) fn is_reconnectable_err(err: &anyhow::Error) -> bool {
    for cause in err.chain() {
        if let Some(SymphoniaError::IoError(io_err)) = cause.downcast_ref::<SymphoniaError>() {
            return io_err.kind() != std::io::ErrorKind::Interrupted;
        }
    }
    false
}

/// Run a decode session with automatic reconnection on I/O errors.
///
/// The `attempt` closure performs one decode session. If it returns an error that
/// `is_reconnectable` classifies as reconnectable, `reconnect` is called (to re-open
/// the stream), and the attempt is retried up to `max_reconnects` times.
///
/// If `stop` is signaled during the retry loop, the loop exits immediately.
pub(crate) fn decode_with_retry(
    attempt: &mut dyn FnMut() -> Result<()>,
    reconnect: &mut dyn FnMut() -> Result<()>,
    is_reconnectable: fn(&anyhow::Error) -> bool,
    max_reconnects: u32,
    stop: &AtomicBool,
) -> Result<()> {
    let mut reconnects = 0u32;
    loop {
        if stop.load(Ordering::Relaxed) {
            // If already stopped, don't attempt a decode session.
            // Return a generic error so the caller doesn't hang.
            anyhow::bail!("stopped");
        }

        match attempt() {
            Ok(()) => return Ok(()),
            Err(e) => {
                if is_reconnectable(&e) && reconnects < max_reconnects {
                    reconnects += 1;
                    tracing::warn!(
                        reconnects,
                        error = %e,
                        "reconnecting after stream error"
                    );
                    reconnect().context("failed to reconnect stream")?;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

struct AudioNormalizer {
    output_rate: u32,
    resampler: Option<StreamResampler>,
}

struct StreamResampler {
    input_rate: u32,
    input_channels: usize,
    resampler: Async<f32>,
}

impl AudioNormalizer {
    fn new(output_rate: u32) -> Self {
        Self {
            output_rate,
            resampler: None,
        }
    }

    fn normalize(
        &mut self,
        spec: &symphonia::core::audio::AudioSpec,
        interleaved: &[f32],
    ) -> Result<Vec<f32>> {
        let input_rate = spec.rate();
        let input_channels = spec.channels().count();
        if input_channels == 0 {
            bail!("stream has no audio channels");
        }

        let samples = if input_rate == self.output_rate {
            interleaved.to_vec()
        } else {
            self.resample_interleaved(interleaved, input_channels, input_rate, self.output_rate)?
        };

        match input_channels {
            1 => {
                let mut stereo = Vec::with_capacity(samples.len() * 2);
                for sample in samples {
                    stereo.push(sample);
                    stereo.push(sample);
                }
                Ok(stereo)
            }
            2 => Ok(samples),
            channels => bail!("stream has unsupported channel count: {channels}"),
        }
    }

    fn resample_interleaved(
        &mut self,
        samples: &[f32],
        channels: usize,
        input_rate: u32,
        output_rate: u32,
    ) -> Result<Vec<f32>> {
        let frames = samples.len() / channels;
        if frames == 0 {
            return Ok(Vec::new());
        }

        let recreate = self.resampler.as_ref().is_none_or(|stream| {
            stream.input_rate != input_rate || stream.input_channels != channels
        });
        if recreate {
            let ratio = output_rate as f64 / input_rate as f64;
            self.resampler = Some(StreamResampler {
                input_rate,
                input_channels: channels,
                resampler: Async::<f32>::new_poly(
                    ratio,
                    1.0,
                    PolynomialDegree::Cubic,
                    frames,
                    channels,
                    FixedAsync::Input,
                )
                .context("failed to create audio resampler")?,
            });
        }

        let input = InterleavedSlice::new(samples, channels, frames)
            .context("failed to adapt decoded audio for resampling")?;
        Ok(self
            .resampler
            .as_mut()
            .expect("resampler initialized")
            .resampler
            .process(&input, 0, None)
            .context("failed to resample audio")?
            .take_data())
    }
}

#[cfg(test)]
fn normalize_audio(
    spec: &symphonia::core::audio::AudioSpec,
    interleaved: &[f32],
) -> Result<Vec<f32>> {
    AudioNormalizer::new(super::audio::DEFAULT_SAMPLE_RATE).normalize(spec, interleaved)
}

#[cfg(test)]
mod tests {
    use super::super::stations;

    #[test]
    #[ignore = "opens live station URLs and decodes network streams"]
    fn built_in_stations_decode_initial_packets() {
        for station in stations::all() {
            let decoded = super::probe_station_decode(*station, 3).unwrap_or_else(|error| {
                panic!(
                    "{} failed decode compatibility probe: {error:#}",
                    station.code
                )
            });
            assert!(decoded > 0, "{} decoded no audio packets", station.code);
        }
    }

    #[test]
    fn normalizes_mono_low_rate_audio_to_stereo_output() {
        use symphonia::core::audio::{AudioSpec, layouts::CHANNEL_LAYOUT_MONO};

        let spec = AudioSpec::new(22_050, CHANNEL_LAYOUT_MONO);
        let mono = vec![0.5; 1_024];
        let output = super::normalize_audio(&spec, &mono).unwrap();

        assert_eq!(output.len() % 2, 0);
        assert!(output.len() > mono.len());
        for frame in output.chunks_exact(2).take(16) {
            assert_eq!(frame[0], frame[1]);
        }
    }

    #[test]
    fn normalizes_to_the_output_device_sample_rate() {
        use symphonia::core::audio::{AudioSpec, layouts::CHANNEL_LAYOUT_MONO};

        let spec = AudioSpec::new(44_100, CHANNEL_LAYOUT_MONO);
        let mono = vec![0.5; 4_410];
        let mut normalizer = super::AudioNormalizer::new(48_000);

        let output = normalizer.normalize(&spec, &mono).unwrap();

        assert_eq!(output.len() % 2, 0);
        assert!(output.len() > mono.len() * 2);
    }

    #[test]
    fn resampling_preserves_filter_state_across_packets() {
        use symphonia::core::audio::{AudioSpec, layouts::CHANNEL_LAYOUT_MONO};

        let spec = AudioSpec::new(48_000, CHANNEL_LAYOUT_MONO);
        let first_packet = vec![0.25; 2_048];
        let second_packet = vec![0.25; 2_048];

        let mut stream_normalizer =
            super::AudioNormalizer::new(super::super::audio::DEFAULT_SAMPLE_RATE);
        let first = stream_normalizer.normalize(&spec, &first_packet).unwrap();
        let second = stream_normalizer.normalize(&spec, &second_packet).unwrap();

        let mut stateless_normalizer =
            super::AudioNormalizer::new(super::super::audio::DEFAULT_SAMPLE_RATE);
        let stateless_second = stateless_normalizer
            .normalize(&spec, &second_packet)
            .unwrap();

        assert_eq!(first.len() % 2, 0);
        assert_eq!(second.len() % 2, 0);
        assert_ne!(second, stateless_second);
    }

    use anyhow::anyhow;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn retry_wraps_decode_attempt_and_reconnects_on_io_error() {
        let mut attempts = 0;
        let mut reconnects = 0;
        let stop = AtomicBool::new(false);

        let result = super::decode_with_retry(
            &mut || {
                attempts += 1;
                if attempts == 1 {
                    Err(anyhow!(symphonia::core::errors::Error::IoError(
                        std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "stream closed")
                    )))
                } else {
                    Ok(())
                }
            },
            &mut || {
                reconnects += 1;
                Ok(())
            },
            super::is_reconnectable_err,
            3,
            &stop,
        );

        assert!(result.is_ok());
        assert_eq!(attempts, 2, "should retry once after first failure");
        assert_eq!(reconnects, 1, "should call reconnect once");
    }

    #[test]
    fn retry_exhausts_max_reconnects_and_bails() {
        let mut attempts = 0;
        let mut reconnects = 0;
        let stop = AtomicBool::new(false);

        let result = super::decode_with_retry(
            &mut || {
                attempts += 1;
                Err(anyhow!(symphonia::core::errors::Error::IoError(
                    std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "stream closed")
                )))
            },
            &mut || {
                reconnects += 1;
                Ok(())
            },
            super::is_reconnectable_err,
            3,
            &stop,
        );

        assert!(result.is_err(), "should fail after exhausting retries");
        assert_eq!(
            attempts, 4,
            "should attempt 4 times (1 initial + 3 retries)"
        );
        assert_eq!(reconnects, 3, "should reconnect 3 times");
    }

    #[test]
    fn non_reconnectable_error_does_not_retry() {
        let mut attempts = 0;
        let mut reconnects = 0;
        let stop = AtomicBool::new(false);

        let result = super::decode_with_retry(
            &mut || {
                attempts += 1;
                Err(anyhow!(symphonia::core::errors::Error::DecodeError(
                    "bad data"
                )))
            },
            &mut || {
                reconnects += 1;
                Ok(())
            },
            super::is_reconnectable_err,
            3,
            &stop,
        );

        assert!(result.is_err());
        assert_eq!(attempts, 1, "should not retry on non-reconnectable error");
        assert_eq!(reconnects, 0, "should not reconnect");
    }

    #[test]
    fn stop_signal_during_retry_loop_exits_early() {
        let stop = AtomicBool::new(true);
        let result = super::decode_with_retry(
            &mut || {
                Err(anyhow!(symphonia::core::errors::Error::IoError(
                    std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "stream closed")
                )))
            },
            &mut || Ok(()),
            super::is_reconnectable_err,
            3,
            &stop,
        );
        assert!(result.is_err());
    }

    #[test]
    fn is_reconnectable_err_identifies_io_errors() {
        use symphonia::core::errors::Error as SErr;

        assert!(super::is_reconnectable_err(&anyhow!(SErr::IoError(
            std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "dropped")
        ))));
        assert!(!super::is_reconnectable_err(&anyhow!(SErr::DecodeError(
            "bad packet"
        ))));
        assert!(!super::is_reconnectable_err(&anyhow!(SErr::Unsupported(
            "some feature"
        ))));
        assert!(!super::is_reconnectable_err(&anyhow!(SErr::IoError(
            std::io::Error::new(std::io::ErrorKind::Interrupted, "interrupted")
        ))));
        assert!(!super::is_reconnectable_err(&anyhow!("some random error")));
    }

    #[test]
    fn resolve_direct_station_url_returns_station_url() {
        let station = stations::find("atma").unwrap();
        let url = super::resolve_station_url(station, None).unwrap();
        assert_eq!(url, station.url);
    }

    #[test]
    #[ignore = "requires yt-dlp on $PATH"]
    fn resolve_youtube_url_via_yt_dlp() {
        let yt_dlp = which::which("yt-dlp").expect("yt-dlp must be on $PATH");
        let station = stations::find("ytlf").unwrap();
        let url = super::resolve_station_url(station, Some(&yt_dlp)).unwrap();
        assert!(
            url.starts_with("http"),
            "resolved YouTube URL should start with http, got: {url}"
        );
    }

    #[test]
    #[ignore = "requires yt-dlp and ffmpeg on $PATH"]
    fn youtube_station_decodes_initial_pcm_samples_via_ffmpeg() {
        let yt_dlp = which::which("yt-dlp").expect("yt-dlp must be on $PATH");
        let ffmpeg = which::which("ffmpeg").expect("ffmpeg must be on $PATH");
        let station = stations::find("ytlf").unwrap();
        let decoded =
            super::probe_youtube_station_via_ffmpeg(station, &yt_dlp, &ffmpeg, 1_024).unwrap();
        assert!(
            decoded >= 1_024,
            "expected at least 1024 samples, got {decoded}"
        );
    }
}
