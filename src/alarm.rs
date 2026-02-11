use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "audio")]
use std::io::BufReader;

/// Plays alarm sound. If an alarm file is provided and the `audio` feature is enabled,
/// uses rodio to play it on loop. Otherwise, emits terminal bell characters.
/// Returns a stop handle that can be used to stop the alarm.
pub fn start_alarm(alarm_file: Option<&str>, duration_secs: u64) -> Arc<AtomicBool> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = stop_flag.clone();

    #[cfg(feature = "audio")]
    if let Some(file_path) = alarm_file {
        let path = file_path.to_string();
        std::thread::spawn(move || {
            play_audio_alarm(&path, duration_secs, &flag_clone);
        });
        return stop_flag;
    }

    #[cfg(not(feature = "audio"))]
    if alarm_file.is_some() {
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        eprintln!(
            "[{}] ⚠ Audio alarm requested but 'audio' feature not enabled. Using terminal bell.",
            now
        );
    }

    std::thread::spawn(move || {
        play_bell_alarm(duration_secs, &flag_clone);
    });

    stop_flag
}

/// Stop the alarm by setting the stop flag
pub fn stop_alarm(stop_flag: &Arc<AtomicBool>) {
    stop_flag.store(true, Ordering::Relaxed);
}

#[cfg(feature = "audio")]
fn play_audio_alarm(file_path: &str, duration_secs: u64, stop_flag: &AtomicBool) {
    let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() else {
        eprintln!("Failed to open audio output, falling back to bell");
        play_bell_alarm(duration_secs, stop_flag);
        return;
    };

    let file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open alarm file '{}': {}", file_path, e);
            play_bell_alarm(duration_secs, stop_flag);
            return;
        }
    };

    let source = match rodio::Decoder::new(BufReader::new(file)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to decode audio: {}", e);
            play_bell_alarm(duration_secs, stop_flag);
            return;
        }
    };

    let sink = match rodio::Sink::try_new(&stream_handle) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create audio sink: {}", e);
            play_bell_alarm(duration_secs, stop_flag);
            return;
        }
    };

    sink.append(rodio::source::Source::repeat_infinite(source));
    sink.play();

    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(duration_secs) {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    sink.stop();
}

fn play_bell_alarm(duration_secs: u64, stop_flag: &AtomicBool) {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(duration_secs) {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        // Terminal bell
        print!("\x07");
        std::thread::sleep(Duration::from_secs(2));
    }
}
