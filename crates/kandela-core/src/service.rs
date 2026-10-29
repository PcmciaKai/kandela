//! The background worker the UI talks to. It owns the connection to the
//! bridge (or the demo home), applies UI commands, and reports changes back
//! through a callback.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::bridge::{self, Bridge, BridgeInfo};
use crate::config::{self, Config};
use crate::demo;
use crate::discovery;
use crate::home::{self, Command, Home};
use crate::pinning;

/// Messages from the UI to the service.
#[derive(Debug, Clone)]
pub enum Request {
    /// Connect to the saved bridge, or start setup if there is none.
    Start,
    Discover,
    /// Wait for the link button on the bridge at this address, then pair.
    Pair {
        address: String,
    },
    CancelPair,
    StartDemo,
    /// Forget the paired bridge and go back to setup.
    Forget,
    Command(Command),
    /// Sent by the pairing task once it has credentials.
    Connect(Config),
}

/// Messages from the service to the UI.
#[derive(Debug, Clone)]
pub enum Update {
    /// "setup", "connecting", "connected", "offline", "untrusted" (the
    /// bridge's certificate changed) or "demo".
    Status {
        status: &'static str,
        bridge_name: String,
        message: String,
    },
    /// JSON-encoded [`home::Snapshot`].
    Home(String),
    Discovering(bool),
    /// JSON-encoded list of [`BridgeInfo`].
    Bridges(String),
    /// "idle", "waiting" or "failed".
    Pairing {
        state: &'static str,
        message: String,
    },
}

type Sink = Arc<dyn Fn(Update) + Send + Sync>;

pub struct Service {
    tx: mpsc::UnboundedSender<Request>,
}

impl Service {
    /// Starts the worker on its own thread with its own async runtime.
    pub fn spawn(on_update: impl Fn(Update) + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let worker_tx = tx.clone();
        std::thread::Builder::new()
            .name("hue-service".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                rt.block_on(
                    async move { Worker::new(Arc::new(on_update), worker_tx).run(rx).await },
                );
            })
            .expect("spawn service thread");
        Self { tx }
    }

    pub fn send(&self, request: Request) {
        let _ = self.tx.send(request);
    }
}

enum Mode {
    Idle,
    Demo,
    Bridge(Bridge),
}

struct Worker {
    sink: Sink,
    tx: mpsc::UnboundedSender<Request>,
    home: Arc<Mutex<Home>>,
    mode: Mode,
    connection: Option<JoinHandle<()>>,
    pairing: Option<JoinHandle<()>>,
    discovery: Option<JoinHandle<()>>,
    outbox: mpsc::UnboundedSender<Outgoing>,
}

/// A command on its way to the bridge.
struct Outgoing {
    key: String,
    bridge: Bridge,
    requests: Vec<home::Request>,
}

fn emit_home(sink: &Sink, home: &Home) {
    match serde_json::to_string(&home.snapshot()) {
        Ok(json) => sink(Update::Home(json)),
        Err(e) => log::error!("snapshot serialisation failed: {e}"),
    }
}

fn status(sink: &Sink, status: &'static str, bridge_name: &str, message: impl Into<String>) {
    sink(Update::Status {
        status,
        bridge_name: bridge_name.to_owned(),
        message: message.into(),
    });
}

fn device_name() -> String {
    let host = std::fs::read_to_string("/etc/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "desktop".into());
    // The bridge allows at most 19 characters after the '#'.
    host.trim().chars().take(19).collect()
}

impl Worker {
    fn new(sink: Sink, tx: mpsc::UnboundedSender<Request>) -> Self {
        let (outbox, outbox_rx) = mpsc::unbounded_channel();
        tokio::spawn(send_loop(outbox_rx, sink.clone()));
        Self {
            sink,
            tx,
            home: Arc::default(),
            mode: Mode::Idle,
            connection: None,
            pairing: None,
            discovery: None,
            outbox,
        }
    }

    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Request>) {
        while let Some(request) = rx.recv().await {
            self.handle(request);
        }
    }

    fn handle(&mut self, request: Request) {
        match request {
            Request::Start => match config::load() {
                Some(cfg) => self.connect(cfg),
                None => self.setup(""),
            },
            Request::Discover => self.discover(),
            Request::Pair { address } => self.pair(address),
            Request::CancelPair => {
                if let Some(p) = self.pairing.take() {
                    p.abort();
                }
                (self.sink)(Update::Pairing {
                    state: "idle",
                    message: String::new(),
                });
            }
            Request::StartDemo => {
                self.disconnect();
                self.mode = Mode::Demo;
                let mut home = self.home.lock().unwrap();
                *home = Home::from_resources(demo::resources());
                emit_home(&self.sink, &home);
                status(&self.sink, "demo", "Demo home", "");
            }
            Request::Forget => {
                config::remove();
                self.setup("");
            }
            Request::Connect(cfg) => {
                if let Some(p) = self.pairing.take() {
                    p.abort();
                }
                (self.sink)(Update::Pairing {
                    state: "idle",
                    message: String::new(),
                });
                self.connect(cfg);
            }
            Request::Command(command) => self.command(command),
        }
    }

    fn disconnect(&mut self) {
        if let Some(c) = self.connection.take() {
            c.abort();
        }
        self.mode = Mode::Idle;
    }

    fn setup(&mut self, message: &str) {
        self.disconnect();
        *self.home.lock().unwrap() = Home::default();
        emit_home(&self.sink, &Home::default());
        status(&self.sink, "setup", "", message);
        self.discover();
    }

    fn discover(&mut self) {
        if self.discovery.as_ref().is_some_and(|d| !d.is_finished()) {
            return;
        }
        let sink = self.sink.clone();
        sink(Update::Discovering(true));
        self.discovery = Some(tokio::spawn(async move {
            let found: Vec<BridgeInfo> = discovery::discover().await;
            sink(Update::Bridges(
                serde_json::to_string(&found).unwrap_or_else(|_| "[]".into()),
            ));
            sink(Update::Discovering(false));
        }));
    }

    fn pair(&mut self, address: String) {
        if let Some(p) = self.pairing.take() {
            p.abort();
        }
        let sink = self.sink.clone();
        let tx = self.tx.clone();
        self.pairing = Some(tokio::spawn(async move {
            let address = address.trim().to_owned();
            sink(Update::Pairing {
                state: "waiting",
                message: String::new(),
            });
            let name = device_name();
            // The link button stays active for 30 seconds after being pressed;
            // give the user a minute to walk over to the bridge.
            for _ in 0..60 {
                match bridge::pair(&address, &name).await {
                    Ok(p) => {
                        let info = bridge::bridge_info(&address).await.ok();
                        let cfg = Config {
                            bridge_id: info.as_ref().map(|i| i.id.clone()).unwrap_or_default(),
                            bridge_name: info
                                .map(|i| i.name)
                                .filter(|n| !n.is_empty())
                                .unwrap_or_else(|| "Hue Bridge".into()),
                            address,
                            app_key: p.app_key,
                            client_key: p.client_key,
                            cert_sha256: Some(pinning::to_hex(&p.certificate)),
                        };
                        if let Err(e) = config::save(&cfg) {
                            log::error!("could not save bridge config: {e}");
                        }
                        let _ = tx.send(Request::Connect(cfg));
                        return;
                    }
                    Err(bridge::Error::LinkButtonNotPressed) => {}
                    Err(e) => {
                        sink(Update::Pairing {
                            state: "failed",
                            message: e.to_string(),
                        });
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            sink(Update::Pairing {
                state: "failed",
                message: "The link button was not pressed in time.".into(),
            });
        }));
    }

    fn connect(&mut self, cfg: Config) {
        self.disconnect();
        let Some(pin) = cfg.cert_sha256.as_deref().and_then(pinning::from_hex) else {
            self.connection = Some(tokio::spawn(learn_certificate(
                cfg,
                self.sink.clone(),
                self.tx.clone(),
            )));
            return;
        };
        let bridge = Bridge::new(&cfg.address, &cfg.app_key, pin);
        self.mode = Mode::Bridge(bridge.clone());
        let sink = self.sink.clone();
        let home = self.home.clone();
        let tx = self.tx.clone();
        self.connection = Some(tokio::spawn(async move {
            let name = cfg.bridge_name;
            let mut backoff = Duration::from_secs(1);
            loop {
                if home.lock().unwrap().is_empty() {
                    status(&sink, "connecting", &name, "");
                }
                match bridge.resources().await {
                    Ok(resources) => {
                        backoff = Duration::from_secs(1);
                        {
                            let mut h = home.lock().unwrap();
                            *h = Home::from_resources(resources);
                            emit_home(&sink, &h);
                        }
                        status(&sink, "connected", &name, "");
                        let result = bridge
                            .stream_events(|events| {
                                let mut h = home.lock().unwrap();
                                let mut changed = false;
                                for e in &events {
                                    changed |= h.apply_event(e);
                                }
                                if changed {
                                    emit_home(&sink, &h);
                                }
                            })
                            .await;
                        if let Err(e) = result {
                            log::warn!("event stream ended: {e}");
                        }
                    }
                    Err(bridge::Error::Unauthorized) => {
                        config::remove();
                        let _ = tx.send(Request::Forget);
                        return;
                    }
                    Err(e @ bridge::Error::CertificateMismatch) => {
                        // Don't retry: the app key must not go to this device.
                        log::warn!("{}: {e}", cfg.address);
                        status(&sink, "untrusted", &name, cfg.address.clone());
                        return;
                    }
                    Err(e) => {
                        status(&sink, "offline", &name, e.to_string());
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(30));
                    }
                }
            }
        }));
    }

    fn command(&mut self, command: Command) {
        let simulate = matches!(self.mode, Mode::Demo);
        let requests = {
            let mut home = self.home.lock().unwrap();
            let requests = home.requests_for(&command);
            home.apply_locally(&command, simulate);
            emit_home(&self.sink, &home);
            requests
        };
        if let Mode::Bridge(bridge) = &self.mode {
            let _ = self.outbox.send(Outgoing {
                key: command.coalesce_key(),
                bridge: bridge.clone(),
                requests,
            });
        }
    }
}

/// Records the certificate of a bridge paired before pinning existed, then
/// connects. Only public information is requested until the certificate is
/// known, and the bridge must still report the id it had when paired.
async fn learn_certificate(mut cfg: Config, sink: Sink, tx: mpsc::UnboundedSender<Request>) {
    status(&sink, "connecting", &cfg.bridge_name, "");
    let mut backoff = Duration::from_secs(1);
    loop {
        match bridge::bridge_info_with_certificate(&cfg.address).await {
            Ok((info, cert)) => {
                if !cfg.bridge_id.is_empty() && info.id != cfg.bridge_id {
                    log::warn!(
                        "{} reports bridge id {}, expected {}",
                        cfg.address,
                        info.id,
                        cfg.bridge_id
                    );
                    status(&sink, "untrusted", &cfg.bridge_name, cfg.address.clone());
                    return;
                }
                cfg.cert_sha256 = Some(pinning::to_hex(&cert));
                if let Err(e) = config::save(&cfg) {
                    log::error!("could not save bridge config: {e}");
                }
                log::info!("pinned the certificate of {}", cfg.address);
                let _ = tx.send(Request::Connect(cfg));
                return;
            }
            Err(e) => {
                status(&sink, "offline", &cfg.bridge_name, e.to_string());
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}

/// Sends commands to the bridge, at most one batch every 100 ms. While a
/// batch is in flight, newer commands with the same key replace older ones,
/// so a dragged slider sends only its latest position.
async fn send_loop(mut rx: mpsc::UnboundedReceiver<Outgoing>, sink: Sink) {
    while let Some(first) = rx.recv().await {
        let mut batch = vec![first];
        while let Ok(next) = rx.try_recv() {
            match batch.iter_mut().find(|o| o.key == next.key) {
                Some(slot) => *slot = next,
                None => batch.push(next),
            }
        }
        for out in batch {
            for req in &out.requests {
                if let Err(e) = out.bridge.send(req).await {
                    log::warn!("{} {} failed: {e}", req.rtype, req.id);
                    let untrusted = matches!(e, bridge::Error::CertificateMismatch);
                    sink(Update::Status {
                        status: if untrusted { "untrusted" } else { "connected" },
                        bridge_name: String::new(),
                        message: e.to_string(),
                    });
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
