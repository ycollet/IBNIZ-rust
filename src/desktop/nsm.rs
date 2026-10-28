use std::{
    env, fs,
    io::{self, Write},
    net::UdpSocket,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use ibniz::Program;
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use url::Url;

pub const NOT_NOW: i32 = -8;
pub const BAD_PROJECT: i32 = -9;
pub const CREATE_FAILED: i32 = -10;
pub const UNSAVED_CHANGES: i32 = -7;
pub const NO_SESSION: i32 = -6;
pub const GENERAL: i32 = -1;
const ANNOUNCE: &str = "/nsm/server/announce";
pub const OPEN: &str = "/nsm/client/open";
pub const SAVE: &str = "/nsm/client/save";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Failure {
    pub code: i32,
    pub message: String,
}

impl Failure {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    pub source: String,
    pub compiled_source: String,
    pub running: bool,
    pub editor: bool,
    pub dc_blocker: bool,
    pub volume: f32,
    pub budget: usize,
    pub slice_ms: u64,
    pub preview_fps: u32,
    pub buffer_ms: u32,
}

impl State {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported NSM state version: {}", self.version));
        }
        if !self.volume.is_finite()
            || !(0.0..=1.0).contains(&self.volume)
            || self.budget < 4
            || !(1..=64).contains(&self.slice_ms)
            || ![15, 30, 60].contains(&self.preview_fps)
            || !(20..=super::audio::MAX_BUFFER_MS).contains(&self.buffer_ms)
        {
            return Err("Invalid playback settings in NSM state".into());
        }
        Program::compile(&self.compiled_source)
            .map_err(|e| format!("Invalid last successfully compiled source in NSM state: {e}"))?;
        Ok(())
    }
}

pub struct Project {
    pub directory: PathBuf,
    pub saved: State,
}

impl Project {
    pub fn open(directory: &Path, initial: State) -> Result<Self, Failure> {
        if !directory.is_absolute() {
            return Err(Failure::new(
                BAD_PROJECT,
                "NSM project path must be absolute",
            ));
        }
        let file = directory.join("state.json");
        let state = match fs::read(&file) {
            Ok(bytes) => {
                let state: State = serde_json::from_slice(&bytes).map_err(|e| {
                    Failure::new(BAD_PROJECT, format!("Cannot read NSM state: {e}"))
                })?;
                state.validate().map_err(|e| Failure::new(BAD_PROJECT, e))?;
                state
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                initial
                    .validate()
                    .map_err(|e| Failure::new(CREATE_FAILED, e))?;
                fs::create_dir_all(directory).map_err(|e| {
                    Failure::new(CREATE_FAILED, format!("Cannot create NSM project: {e}"))
                })?;
                write_state(&file, &initial).map_err(|e| {
                    Failure::new(CREATE_FAILED, format!("Cannot create NSM state: {e}"))
                })?;
                initial
            }
            Err(error) => {
                return Err(Failure::new(
                    BAD_PROJECT,
                    format!("Cannot read NSM state: {error}"),
                ));
            }
        };
        Ok(Self {
            directory: directory.into(),
            saved: state,
        })
    }

    pub fn save(&mut self, state: State) -> Result<(), Failure> {
        state.validate().map_err(|e| Failure::new(GENERAL, e))?;
        write_state(&self.directory.join("state.json"), &state)
            .map_err(|e| Failure::new(GENERAL, format!("Cannot save NSM state: {e}")))?;
        self.saved = state;
        Ok(())
    }
}

fn write_state(path: &Path, state: &State) -> io::Result<()> {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    let temporary = path.with_file_name(format!(
        ".state-{}-{}.tmp",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed),
    ));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if let Err(error) = result {
        if let Err(cleanup) = fs::remove_file(&temporary) {
            return Err(io::Error::other(format!(
                "{error}; cannot remove {}: {cleanup}",
                temporary.display(),
            )));
        }
        return Err(error);
    }
    Ok(())
}

#[derive(Debug)]
pub enum Event {
    Announced,
    Open {
        path: String,
        display: String,
        client_id: String,
    },
    Save,
    Warning(String),
}

#[derive(PartialEq)]
enum Phase {
    Waiting,
    Active,
    Unavailable,
}

pub struct Client {
    socket: UdpSocket,
    phase: Phase,
    announced_at: Instant,
    pub manager: String,
    last_dirty: Option<bool>,
    terminate: Arc<AtomicBool>,
    #[cfg(unix)]
    signal: Option<signal_hook::SigId>,
}

impl Client {
    pub fn from_environment() -> Result<Option<Self>, String> {
        let url = match env::var("NSM_URL") {
            Ok(url) => url,
            Err(env::VarError::NotPresent) => return Ok(None),
            Err(error) => return Err(format!("Invalid NSM_URL: {error}")),
        };
        let executable = env::args_os()
            .next()
            .ok_or("Missing executable name for NSM announce")?
            .into_string()
            .map_err(|_| "NSM executable name is not UTF-8")?;
        Self::connect(&url, &executable).map(Some)
    }

    fn connect(url: &str, executable: &str) -> Result<Self, String> {
        let url = Url::parse(url).map_err(|e| format!("Invalid NSM_URL: {e}"))?;
        if url.scheme() != "osc.udp"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_none_or(|port| port == 0)
            || !matches!(url.path(), "" | "/")
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("NSM_URL must be osc.udp://host:port/ (TCP is not supported)".into());
        }
        let mut addresses = url
            .socket_addrs(|| None)
            .map_err(|e| format!("Cannot resolve NSM_URL: {e}"))?;
        addresses.sort_by_key(|address| address.is_ipv6());
        let mut connection = None;
        let mut errors = Vec::new();
        for address in addresses {
            let result = (|| -> io::Result<UdpSocket> {
                let socket = UdpSocket::bind(if address.is_ipv6() {
                    "[::]:0"
                } else {
                    "0.0.0.0:0"
                })?;
                socket.connect(address)?;
                socket.set_nonblocking(true)?;
                Ok(socket)
            })();
            match result {
                Ok(socket) => {
                    connection = Some(socket);
                    break;
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
        let socket =
            connection.ok_or_else(|| format!("Cannot connect to NSM: {}", errors.join("; ")))?;
        let terminate = Arc::new(AtomicBool::new(false));
        let mut client = Self {
            socket,
            phase: Phase::Waiting,
            announced_at: Instant::now(),
            manager: String::new(),
            last_dirty: None,
            terminate,
            #[cfg(unix)]
            signal: None,
        };
        client.send(
            ANNOUNCE,
            vec![
                OscType::String("IBNIZ Rust".into()),
                OscType::String(":dirty:".into()),
                OscType::String(executable.into()),
                OscType::Int(1),
                OscType::Int(1),
                OscType::Int(
                    i32::try_from(std::process::id())
                        .map_err(|_| "NSM PID exceeds OSC integer range")?,
                ),
            ],
        )?;
        client.install_termination_handler()?;
        Ok(client)
    }

    fn install_termination_handler(&mut self) -> Result<(), String> {
        #[cfg(unix)]
        {
            self.signal = Some(
                signal_hook::flag::register(signal_hook::consts::SIGTERM, self.terminate.clone())
                    .map_err(|e| format!("Cannot register NSM SIGTERM handler: {e}"))?,
            );
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn request_termination(&self) {
        self.terminate.store(true, Ordering::Relaxed);
    }

    pub fn active(&self) -> bool {
        self.phase == Phase::Active
    }

    pub fn needs_poll(&self) -> bool {
        self.phase != Phase::Unavailable
    }

    pub fn termination_requested(&self) -> bool {
        self.terminate.load(Ordering::Relaxed)
    }

    fn send(&self, address: &str, args: Vec<OscType>) -> Result<(), String> {
        let bytes = rosc::encoder::encode(&OscPacket::Message(OscMessage {
            addr: address.into(),
            args,
        }))
        .map_err(|e| format!("Cannot encode NSM message: {e}"))?;
        self.socket
            .send(&bytes)
            .map_err(|e| format!("Cannot send NSM message: {e}"))?;
        Ok(())
    }

    pub fn reply(&self, path: &str, result: Result<(), Failure>) -> Result<(), String> {
        match result {
            Ok(()) => self.send(
                "/reply",
                vec![OscType::String(path.into()), OscType::String("OK".into())],
            ),
            Err(error) => self.send(
                "/error",
                vec![
                    OscType::String(path.into()),
                    OscType::Int(error.code),
                    OscType::String(error.message),
                ],
            ),
        }
    }

    pub fn report_dirty(&mut self, dirty: bool) -> Result<(), String> {
        if self.active() && self.last_dirty != Some(dirty) {
            self.send(
                if dirty {
                    "/nsm/client/is_dirty"
                } else {
                    "/nsm/client/is_clean"
                },
                vec![],
            )?;
            self.last_dirty = Some(dirty);
        }
        Ok(())
    }

    pub fn poll(&mut self) -> Vec<Event> {
        if !self.needs_poll() {
            return Vec::new();
        }
        let mut events = Vec::new();
        let mut buffer = [0_u8; 65536];
        for _ in 0..32 {
            let size = match self.socket.recv(&mut buffer) {
                Ok(size) => size,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    if !self.active() {
                        self.phase = Phase::Unavailable;
                    }
                    events.push(Event::Warning(format!("NSM receive failed: {error}")));
                    break;
                }
            };
            match rosc::decoder::decode_udp(&buffer[..size]) {
                Ok(([], packet)) => self.packet(packet, 0, &mut events),
                Ok(_) => events.push(Event::Warning("Trailing data in NSM OSC packet".into())),
                Err(error) => {
                    events.push(Event::Warning(format!("Invalid NSM OSC packet: {error}")))
                }
            }
        }
        if self.phase == Phase::Waiting && self.announced_at.elapsed() >= HANDSHAKE_TIMEOUT {
            self.phase = Phase::Unavailable;
            events.push(Event::Warning(
                "NSM announce timed out; continuing without session management".into(),
            ));
        }
        events
    }

    fn packet(&mut self, packet: OscPacket, depth: usize, events: &mut Vec<Event>) {
        if depth > 8 || events.len() >= 128 {
            events.push(Event::Warning(
                "NSM OSC bundle exceeds processing limit".into(),
            ));
            return;
        }
        match packet {
            OscPacket::Bundle(bundle) => {
                for packet in bundle.content {
                    self.packet(packet, depth + 1, events);
                    if events.len() >= 128 {
                        break;
                    }
                }
            }
            OscPacket::Message(message) => match (message.addr.as_str(), message.args.as_slice()) {
                (
                    "/reply",
                    [
                        OscType::String(path),
                        OscType::String(_),
                        OscType::String(manager),
                        OscType::String(_),
                    ],
                ) if path == ANNOUNCE && self.phase == Phase::Waiting => {
                    self.phase = Phase::Active;
                    self.manager = manager.clone();
                    events.push(Event::Announced);
                }
                (
                    "/error",
                    [
                        OscType::String(path),
                        OscType::Int(code),
                        OscType::String(message),
                    ],
                ) if path == ANNOUNCE && self.phase == Phase::Waiting => {
                    self.phase = Phase::Unavailable;
                    events.push(Event::Warning(format!(
                        "NSM announce rejected ({code}): {message}"
                    )));
                }
                (
                    OPEN,
                    [
                        OscType::String(path),
                        OscType::String(display),
                        OscType::String(client_id),
                    ],
                ) if self.active() => {
                    events.push(Event::Open {
                        path: path.clone(),
                        display: display.clone(),
                        client_id: client_id.clone(),
                    });
                }
                (SAVE, []) if self.active() => events.push(Event::Save),
                ("/nsm/client/session_is_loaded", []) => {}
                ("/reply", [OscType::String(path), ..]) if path == ANNOUNCE && self.active() => {}
                (OPEN | SAVE, _) => {
                    let error =
                        Failure::new(NOT_NOW, "Invalid NSM request or announce not yet accepted");
                    if let Err(error) = self.reply(&message.addr, Err(error)) {
                        events.push(Event::Warning(error));
                    }
                }
                _ => events.push(Event::Warning(format!(
                    "Unsupported NSM message: {}",
                    message.addr
                ))),
            },
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(signal) = self.signal {
            signal_hook::low_level::unregister(signal);
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::{
        net::SocketAddr,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    pub struct TestDirectory(pub PathBuf);

    impl TestDirectory {
        pub fn new() -> Self {
            static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = env::temp_dir().join(format!(
                "ibniz-nsm-test-{}-{timestamp}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    pub struct Server {
        socket: UdpSocket,
        pub peer: SocketAddr,
    }

    impl Server {
        pub fn pair() -> (Self, Client) {
            let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let client = Client::connect(
                &format!("osc.udp://{}/", socket.local_addr().unwrap()),
                "/custom/bin/ibniz-wrapper",
            )
            .unwrap();
            let mut bytes = [0; 65536];
            let (size, peer) = socket.recv_from(&mut bytes).unwrap();
            let (_, OscPacket::Message(announce)) =
                rosc::decoder::decode_udp(&bytes[..size]).unwrap()
            else {
                panic!("expected announce message");
            };
            assert_eq!(announce.addr, ANNOUNCE);
            assert_eq!(
                announce.args,
                vec![
                    OscType::String("IBNIZ Rust".into()),
                    OscType::String(":dirty:".into()),
                    OscType::String("/custom/bin/ibniz-wrapper".into()),
                    OscType::Int(1),
                    OscType::Int(1),
                    OscType::Int(std::process::id() as i32),
                ]
            );
            (Self { socket, peer }, client)
        }

        pub fn send(&self, address: &str, args: Vec<OscType>) {
            let bytes = rosc::encoder::encode(&OscPacket::Message(OscMessage {
                addr: address.into(),
                args,
            }))
            .unwrap();
            self.socket.send_to(&bytes, self.peer).unwrap();
        }

        pub fn receive(&self) -> OscMessage {
            let mut bytes = [0; 65536];
            let (size, peer) = self.socket.recv_from(&mut bytes).unwrap();
            assert_eq!(
                peer, self.peer,
                "every NSM message must use the announce socket"
            );
            let (_, OscPacket::Message(message)) =
                rosc::decoder::decode_udp(&bytes[..size]).unwrap()
            else {
                panic!("expected OSC message");
            };
            message
        }

        pub fn receive_after(&self, mut pump: impl FnMut()) -> OscMessage {
            self.socket.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut bytes = [0; 65536];
            let (size, peer) = loop {
                pump();
                match self.socket.recv_from(&mut bytes) {
                    Ok(packet) => break packet,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "NSM client did not respond");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("NSM server receive failed: {error}"),
                }
            };
            self.socket.set_nonblocking(false).unwrap();
            assert_eq!(peer, self.peer);
            let (_, OscPacket::Message(message)) =
                rosc::decoder::decode_udp(&bytes[..size]).unwrap()
            else {
                panic!("expected OSC message");
            };
            message
        }

        pub fn accept(&self, client: &mut Client) {
            self.send(
                "/reply",
                vec![
                    OscType::String(ANNOUNCE.into()),
                    OscType::String("Welcome".into()),
                    OscType::String("Mock NSM".into()),
                    OscType::String(":optional-gui:".into()),
                ],
            );
            let deadline = Instant::now() + Duration::from_secs(2);
            while !client.active() {
                assert!(Instant::now() < deadline, "NSM handshake did not complete");
                let events = client.poll();
                assert!(
                    events.iter().all(|event| matches!(event, Event::Announced)),
                    "{events:?}"
                );
                thread::sleep(Duration::from_millis(1));
            }
        }

        pub fn open(&self, path: &Path) {
            self.send(
                OPEN,
                vec![
                    OscType::String(path.to_str().unwrap().into()),
                    OscType::String("Example session".into()),
                    OscType::String("IBNIZ Rust.nTEST".into()),
                ],
            );
        }
    }

    pub fn state() -> State {
        State {
            version: 1,
            source: "^xp M d3r15&*".into(),
            compiled_source: "^xp M d3r15&*".into(),
            running: false,
            editor: true,
            dc_blocker: true,
            volume: 0.2,
            budget: 4_000_000,
            slice_ms: 6,
            preview_fps: 60,
            buffer_ms: 80,
        }
    }

    #[test]
    fn playback_settings_accept_expanded_limits_and_reject_values_above_them() {
        let mut state = state();
        state.slice_ms = 64;
        state.buffer_ms = 1024;
        assert!(state.validate().is_ok());
        state.slice_ms = 65;
        assert!(state.validate().is_err());
        state.slice_ms = 64;
        state.buffer_ms = 1025;
        assert!(state.validate().is_err());
    }

    #[test]
    fn handshake_requests_replies_and_dirty_notifications_share_the_same_socket() {
        let (server, mut client) = Server::pair();
        assert!(!client.active());
        server.send(SAVE, vec![]);
        let error = server.receive_after(|| {
            client.poll();
        });
        assert_eq!(error.addr, "/error");
        assert_eq!(error.args[1], OscType::Int(NOT_NOW));
        server.accept(&mut client);
        client.report_dirty(false).unwrap();
        assert_eq!(server.receive().addr, "/nsm/client/is_clean");
        client.report_dirty(true).unwrap();
        assert_eq!(server.receive().addr, "/nsm/client/is_dirty");
        client.report_dirty(true).unwrap();
        client.reply(OPEN, Ok(())).unwrap();
        let reply = server.receive();
        assert_eq!(reply.addr, "/reply");
        assert_eq!(reply.args[0], OscType::String(OPEN.into()));
        client
            .reply(SAVE, Err(Failure::new(GENERAL, "write failed")))
            .unwrap();
        let error = server.receive();
        assert_eq!(error.addr, "/error");
        assert_eq!(
            error.args,
            vec![
                OscType::String(SAVE.into()),
                OscType::Int(GENERAL),
                OscType::String("write failed".into()),
            ]
        );
    }

    #[test]
    fn invalid_urls_and_unanswered_announces_fall_back_without_management() {
        for url in [
            "garbage",
            "osc.tcp://localhost:1234/",
            "osc.udp://localhost/",
            "osc.udp://localhost:0/",
            "osc.udp://user@localhost:1234/",
            "osc.udp://localhost:1234/path",
        ] {
            assert!(Client::connect(url, "ibniz").is_err(), "{url}");
        }
        let (_server, mut client) = Server::pair();
        client.announced_at = Instant::now() - HANDSHAKE_TIMEOUT - Duration::from_secs(1);
        assert!(
            client
                .poll()
                .iter()
                .any(|event| matches!(event, Event::Warning(error) if error.contains("timed out")))
        );
        assert!(!client.active());
        assert!(!client.needs_poll());
    }

    #[test]
    fn malformed_requests_report_errors_and_foreign_sockets_are_ignored() {
        let (server, mut client) = Server::pair();
        server.accept(&mut client);
        server.send(OPEN, vec![OscType::Int(1)]);
        assert_eq!(
            server
                .receive_after(|| {
                    client.poll();
                })
                .addr,
            "/error"
        );
        let foreign = UdpSocket::bind("127.0.0.1:0").unwrap();
        foreign.send_to(b"invalid OSC", server.peer).unwrap();
        assert!(client.poll().is_empty());
        server.socket.send_to(b"invalid OSC", server.peer).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while client.poll().is_empty() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        assert!(client.active());
    }

    #[test]
    fn announce_errors_only_disable_clients_before_handshake_acceptance() {
        for accepted in [false, true] {
            let (server, mut client) = Server::pair();
            if accepted {
                server.accept(&mut client);
            }
            server.send(
                "/error",
                vec![
                    OscType::String(ANNOUNCE.into()),
                    OscType::Int(GENERAL),
                    OscType::String("announce failed".into()),
                ],
            );
            let deadline = Instant::now() + Duration::from_secs(2);
            let events = loop {
                let events = client.poll();
                if !events.is_empty() {
                    break events;
                }
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(1));
            };
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, Event::Warning(_)))
            );
            assert_eq!(client.active(), accepted);
            assert_eq!(client.needs_poll(), accepted);
        }
    }

    #[test]
    fn project_round_trip_is_self_contained_and_validates_before_replacing_state() {
        let directory = TestDirectory::new();
        let path = directory.0.join("client.nTEST");
        let mut project = Project::open(&path, state()).unwrap();
        assert!(path.join("state.json").exists());
        let mut updated = state();
        updated.volume = 0.7;
        updated.preview_fps = 15;
        updated.source = "invalid".into();
        project.save(updated.clone()).unwrap();
        let reopened = Project::open(&path, state()).unwrap();
        assert_eq!(reopened.saved, updated);
        let bytes = fs::read(path.join("state.json")).unwrap();
        let mut invalid = updated.clone();
        invalid.preview_fps = 0;
        assert!(project.save(invalid).is_err());
        assert_eq!(project.saved, updated);
        assert_eq!(fs::read(path.join("state.json")).unwrap(), bytes);
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
        fs::write(path.join("state.json"), b"not JSON").unwrap();
        assert_eq!(
            Project::open(&path, state()).err().unwrap().code,
            BAD_PROJECT
        );
        assert_eq!(fs::read(path.join("state.json")).unwrap(), b"not JSON");
        assert!(Project::open(Path::new("relative-project"), state()).is_err());
    }

    #[test]
    fn failed_atomic_replacement_preserves_saved_baseline_and_removes_temporary_file() {
        let directory = TestDirectory::new();
        let path = directory.0.join("client.nTEST");
        let mut project = Project::open(&path, state()).unwrap();
        let before = fs::read(path.join("state.json")).unwrap();
        fs::rename(path.join("state.json"), path.join("backup.json")).unwrap();
        fs::create_dir(path.join("state.json")).unwrap();
        let mut changed = state();
        changed.volume = 0.8;
        assert!(project.save(changed).is_err());
        assert_eq!(project.saved, state());
        assert_eq!(fs::read(path.join("backup.json")).unwrap(), before);
        assert_eq!(fs::read_dir(path).unwrap().count(), 2);
    }
}
