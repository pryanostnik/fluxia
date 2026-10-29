//! Аудио-движок: отдельный поток с rodio, общение через каналы.

use std::io::Cursor;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, Player};

pub enum Cmd {
    Load(Arc<[u8]>, Duration),
    Toggle,
    Seek(Duration),
    Volume(f32),
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub position: Duration,
    pub duration: Duration,
    pub playing: bool,
    pub loaded: bool,
}

pub struct Audio {
    tx: Sender<Cmd>,
    snap: Arc<Mutex<Snapshot>>,
    finished: Arc<Mutex<bool>>,
    error: Arc<Mutex<Option<String>>>,
}

impl Audio {
    pub fn new(volume: f32) -> Self {
        let (tx, rx) = mpsc::channel();
        let snap = Arc::new(Mutex::new(Snapshot::default()));
        let finished = Arc::new(Mutex::new(false));
        let error = Arc::new(Mutex::new(None));

        let (s, f, e) = (snap.clone(), finished.clone(), error.clone());
        std::thread::Builder::new()
            .name("fluxia-audio".into())
            .spawn(move || run(rx, s, f, e, volume))
            .ok();

        Self {
            tx,
            snap,
            finished,
            error,
        }
    }

    pub fn load(&self, bytes: Arc<[u8]>, duration: Duration) {
        let _ = self.tx.send(Cmd::Load(bytes, duration));
    }

    pub fn toggle(&self) {
        let _ = self.tx.send(Cmd::Toggle);
    }

    pub fn seek(&self, pos: Duration) {
        let _ = self.tx.send(Cmd::Seek(pos));
    }

    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(Cmd::Volume(v));
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snap.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Возвращает `true` ровно один раз после окончания трека.
    pub fn take_finished(&self) -> bool {
        match self.finished.lock() {
            Ok(mut f) => std::mem::take(&mut *f),
            Err(_) => false,
        }
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|e| e.clone())
    }
}

fn run(
    rx: Receiver<Cmd>,
    snap: Arc<Mutex<Snapshot>>,
    finished: Arc<Mutex<bool>>,
    error: Arc<Mutex<Option<String>>>,
    volume: f32,
) {
    let sink = match DeviceSinkBuilder::open_default_sink() {
        Ok(sink) => sink,
        Err(e) => {
            if let Ok(mut slot) = error.lock() {
                *slot = Some(format!("Аудиоустройство недоступно: {e}"));
            }
            // продолжаем жить, чтобы UI не висел
            while rx.recv().is_ok() {}
            return;
        }
    };

    let player = Player::connect_new(sink.mixer());
    player.set_volume(volume);

    let mut bytes: Option<Arc<[u8]>> = None;
    let mut duration = Duration::ZERO;
    let mut active = false;

    loop {
        let cmd = rx.recv_timeout(Duration::from_millis(60));
        match cmd {
            Ok(Cmd::Load(data, dur)) => {
                duration = dur;
                bytes = Some(data.clone());
                player.clear();
                if let Ok(decoder) = Decoder::new(Cursor::new(data)) {
                    player.append(decoder);
                    player.play();
                    active = true;
                    if let Ok(mut f) = finished.lock() {
                        *f = false;
                    }
                }
            }
            Ok(Cmd::Toggle) => {
                if player.is_paused() {
                    player.play();
                    active = true;
                } else {
                    player.pause();
                }
            }
            Ok(Cmd::Seek(pos)) => {
                if player.try_seek(pos).is_err() {
                    // запасной путь: пересоздаём поток и ищем заново
                    if let Some(data) = bytes.clone() {
                        player.clear();
                        if let Ok(decoder) = Decoder::new(Cursor::new(data)) {
                            player.append(decoder);
                            player.play();
                            let _ = player.try_seek(pos);
                            active = true;
                        }
                    }
                }
            }
            Ok(Cmd::Volume(v)) => player.set_volume(v),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let empty = player.empty();
        let paused = player.is_paused();
        let pos = player.get_pos().min(if duration.is_zero() { player.get_pos() } else { duration });

        if let Ok(mut s) = snap.lock() {
            s.position = pos;
            s.duration = duration;
            s.loaded = duration > Duration::ZERO;
            s.playing = !paused && !empty;
        }

        if empty && active {
            active = false;
            if let Ok(mut f) = finished.lock() {
                *f = true;
            }
        }
    }
}
