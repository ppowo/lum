mod audio;
mod controls;
mod decode;
pub mod stations;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use tokio::task::JoinHandle;

use crate::cli::RadioArgs;
use audio::AudioPlayer;
use controls::{ControlEvent, RawMode};
use stations::Station;

pub async fn run(args: RadioArgs) -> Result<()> {
    match args.station {
        None => {
            println!("{}", stations::format_listing());
            Ok(())
        }
        Some(code) => {
            let station = stations::find(&code).with_context(|| {
                format!(
                    "station not found: {code}\n\n{}",
                    stations::format_listing()
                )
            })?;
            play(*station).await
        }
    }
}

async fn play(station: Station) -> Result<()> {
    let audio = AudioPlayer::open()?;
    println!(
        "Now playing\n  {:<4}  {}\n",
        station.code, station.description
    );
    println!("space/p pause · q/ctrl+c stop");

    let raw = RawMode::enter()
        .inspect_err(|error| tracing::warn!(error = %error, "keyboard unavailable"))?;
    let mut controls = controls::spawn_control_task();

    let mut paused = false;
    let mut stop = Arc::new(AtomicBool::new(false));
    let mut task = Some(start_decode_task(
        station,
        audio.writer(),
        Arc::clone(&stop),
    ));

    loop {
        tokio::select! {
            control = controls.recv() => {
                match control {
                    Some(ControlEvent::Stop) | None => break,
                    Some(ControlEvent::TogglePause) => {
                        if paused {
                            stop = Arc::new(AtomicBool::new(false));
                            task = Some(start_decode_task(station, audio.writer(), Arc::clone(&stop)));
                            paused = false;
                            print!("\rplaying · press space to pause       ");
                        } else {
                            stop.store(true, Ordering::Release);
                            audio.clear();
                            if let Some(handle) = task.take() {
                                let _ = handle.await;
                            }
                            paused = true;
                            print!("\rpaused  · press space to resume      ");
                        }
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                }
            }
            result = async { task.as_mut().unwrap().await }, if task.is_some() && !paused => {
                task = None;
                match result {
                    Ok(Ok(())) => {
                        eprintln!("\nstream ended");
                        break;
                    }
                    Ok(Err(error)) => return Err(error),
                    Err(error) => return Err(error).context("decode task failed"),
                }
            }
        }
    }

    stop.store(true, Ordering::Release);
    audio.clear();
    if let Some(handle) = task {
        let _ = handle.await;
    }
    drop(raw);
    println!("\nstopping...");
    println!("stopped");
    Ok(())
}

fn start_decode_task(
    station: Station,
    writer: audio::AudioWriter,
    stop: Arc<AtomicBool>,
) -> JoinHandle<Result<()>> {
    tokio::task::spawn_blocking(move || decode::stream_station(station, writer, stop))
}
