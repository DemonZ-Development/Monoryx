use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::app::events::AppEvent;

const RESTORE_REQUEST: &[u8] = b"MONORYX_RESTORE\n";
const RESTORE_ACK: &[u8] = b"MONORYX_ACK\n";
const IMPORT_CAPABILITY: &[u8] = b"MONORYX_IMPORT_V1\n";
const IMPORT_REQUEST: &[u8] = b"MONORYX_OPEN_PACKS ";
const IMPORT_ACK: &[u8] = b"MONORYX_PACKS_ACK\n";
const MAX_REQUEST_BYTES: usize = 65_536;
const MAX_PACKS: usize = 16;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn modpack_arguments(args: impl IntoIterator<Item = OsString>) -> Vec<PathBuf> {
    let mut packs = Vec::new();
    let mut positional_only = false;
    for argument in args {
        if !positional_only {
            if argument == "--" {
                positional_only = true;
                continue;
            }
            if argument.as_encoded_bytes().starts_with(b"-") {
                continue;
            }
        }
        let path = PathBuf::from(argument);
        if !is_modpack_path(&path) || !path.is_file() {
            continue;
        }
        if let Ok(path) = path.canonicalize() {
            if !packs.contains(&path) {
                packs.push(path);
                if packs.len() == MAX_PACKS {
                    break;
                }
            }
        }
    }
    packs
}

fn is_modpack_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mrpack"))
}

fn encode_import(packs: &[PathBuf]) -> io::Result<Vec<u8>> {
    validate_paths(packs)?;
    let mut frame = IMPORT_REQUEST.to_vec();
    serde_json::to_writer(&mut frame, packs).map_err(io::Error::other)?;
    frame.push(b'\n');
    if frame.len() > MAX_REQUEST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Modpack request exceeds the maximum size",
        ));
    }
    Ok(frame)
}

fn decode_import(frame: &[u8]) -> io::Result<Vec<PathBuf>> {
    if frame.len() > MAX_REQUEST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Modpack request exceeds the maximum size",
        ));
    }
    let payload = frame
        .strip_prefix(IMPORT_REQUEST)
        .and_then(|payload| payload.strip_suffix(b"\n"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid modpack request"))?;
    let packs: Vec<PathBuf> = serde_json::from_slice(payload).map_err(io::Error::other)?;
    validate_paths(&packs)?;
    Ok(packs)
}

fn validate_paths(packs: &[PathBuf]) -> io::Result<()> {
    if packs.is_empty()
        || packs.len() > MAX_PACKS
        || packs
            .iter()
            .any(|path| !path.is_absolute() || !is_modpack_path(path))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Modpack request must contain absolute .mrpack paths",
        ));
    }
    Ok(())
}

fn read_frame(reader: &mut BufReader<TcpStream>, deadline: Instant) -> io::Result<Vec<u8>> {
    let mut frame = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Request timed out"))?;
        reader.get_ref().set_read_timeout(Some(remaining))?;
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Incomplete request",
            ));
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |position| position + 1);
        if frame.len() + count > MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Request exceeds the maximum size",
            ));
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count);
        if newline.is_some() {
            return Ok(frame);
        }
    }
}

fn forward_to(address: SocketAddr, packs: &[PathBuf]) -> io::Result<()> {
    let request = if packs.is_empty() {
        None
    } else {
        Some(encode_import(packs)?)
    };
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(600))?;
    stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
    stream.write_all(RESTORE_REQUEST)?;
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let mut reader = BufReader::new(stream);
    if read_frame(&mut reader, deadline)? != RESTORE_ACK {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unexpected restore acknowledgement",
        ));
    }
    if let Some(request) = request {
        if read_frame(&mut reader, deadline)? != IMPORT_CAPABILITY {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Running launcher does not accept modpack requests",
            ));
        }
        reader.get_mut().write_all(&request)?;
        if read_frame(&mut reader, deadline)? != IMPORT_ACK {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unexpected modpack acknowledgement",
            ));
        }
    }
    Ok(())
}

pub(crate) fn forward_request(port: u16, packs: &[PathBuf]) -> bool {
    forward_to(SocketAddr::from(([127, 0, 0, 1], port)), packs).is_ok()
}

fn receive_request(stream: TcpStream, mut send: impl FnMut(AppEvent) -> bool) -> io::Result<()> {
    stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let mut reader = BufReader::new(stream);
    if read_frame(&mut reader, deadline)? != RESTORE_REQUEST {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid restore request",
        ));
    }
    if !send(AppEvent::RestoreWindow) {
        return Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Launcher is closing",
        ));
    }
    reader.get_mut().write_all(RESTORE_ACK)?;
    reader.get_mut().write_all(IMPORT_CAPABILITY)?;
    let frame = read_frame(&mut reader, deadline)?;
    for path in decode_import(&frame)? {
        if !send(AppEvent::OpenModpack(path)) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Launcher is closing",
            ));
        }
    }
    reader.get_mut().write_all(IMPORT_ACK)
}

pub(crate) fn serve_requests(listener: TcpListener, tx: Sender<AppEvent>, ctx: egui::Context) {
    if listener.set_nonblocking(false).is_err() {
        return;
    }
    while let Ok((stream, _)) = listener.accept() {
        let _ = receive_request(stream, |event| {
            if tx.send(event).is_err() {
                return false;
            }
            ctx.request_repaint();
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn arguments_resolve_relative_paths_and_ignore_invalid_files() {
        let current_dir = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir_in(&current_dir).unwrap();
        let pack = dir.path().join("Example Pack.MRPACK");
        let other = dir.path().join("readme.txt");
        std::fs::write(&pack, b"pack").unwrap();
        std::fs::write(&other, b"readme").unwrap();
        let relative = pack.strip_prefix(&current_dir).unwrap();
        let paths = modpack_arguments([
            OsString::from("--flag"),
            OsString::from(relative),
            OsString::from(&pack),
            OsString::from(other),
            OsString::from(dir.path().join("missing.mrpack")),
            OsString::from(dir.path()),
        ]);
        assert_eq!(paths, vec![pack.canonicalize().unwrap()]);
    }

    #[test]
    fn import_protocol_preserves_spaces_unicode_and_multiple_paths() {
        let dir = tempfile::tempdir().unwrap();
        let packs = vec![
            dir.path().join("Example Pack.mrpack"),
            dir.path().join("世界.MRPACK"),
        ];
        assert_eq!(
            decode_import(&encode_import(&packs).unwrap()).unwrap(),
            packs
        );
    }

    #[test]
    fn import_protocol_rejects_invalid_and_oversized_requests() {
        assert!(decode_import(b"MONORYX_OPEN_PACKS []\n").is_err());
        assert!(decode_import(b"MONORYX_OPEN_PACKS [\"relative.mrpack\"]\n").is_err());
        assert!(decode_import(b"MONORYX_OPEN_PACKS [\"relative.txt\"]\n").is_err());
        assert!(decode_import(b"MONORYX_OPEN_PACKS [\"broken\"\n").is_err());
        assert!(decode_import(b"UNKNOWN []\n").is_err());
        let dir = tempfile::tempdir().unwrap();
        let too_many = vec![dir.path().join("pack.mrpack"); MAX_PACKS + 1];
        assert!(encode_import(&too_many).is_err());
        let too_large = dir
            .path()
            .join(format!("{}.mrpack", "a".repeat(MAX_REQUEST_BYTES)));
        assert!(encode_import(&[too_large]).is_err());
        assert!(decode_import(&vec![b'a'; MAX_REQUEST_BYTES + 1]).is_err());
    }

    #[test]
    fn import_forwarding_reaches_the_primary_launcher() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            receive_request(stream, |event| tx.send(event).is_ok()).unwrap();
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Pack With Spaces.mrpack");
        forward_to(address, std::slice::from_ref(&path)).unwrap();
        assert!(matches!(rx.recv().unwrap(), AppEvent::RestoreWindow));
        assert!(matches!(rx.recv().unwrap(), AppEvent::OpenModpack(received) if received == path));
        server.join().unwrap();
    }

    #[test]
    fn restore_ack_remains_compatible_with_older_clients() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            receive_request(stream, |event| matches!(event, AppEvent::RestoreWindow)).unwrap_err();
        });
        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(RESTORE_REQUEST).unwrap();
        let mut response = [0; RESTORE_ACK.len()];
        client.read_exact(&mut response).unwrap();
        assert_eq!(response, RESTORE_ACK);
        drop(client);
        server.join().unwrap();
    }

    #[test]
    fn an_older_primary_accepts_restore_but_cannot_drop_imports_silently() {
        for importing in [false, true] {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; RESTORE_REQUEST.len()];
                stream.read_exact(&mut request).unwrap();
                assert_eq!(request, RESTORE_REQUEST);
                stream.write_all(RESTORE_ACK).unwrap();
            });
            let dir = tempfile::tempdir().unwrap();
            let packs = if importing {
                vec![dir.path().join("pack.mrpack")]
            } else {
                Vec::new()
            };
            assert_eq!(forward_to(address, &packs).is_ok(), !importing);
            server.join().unwrap();
        }
    }

    #[test]
    fn stalled_clients_expire_without_an_unbounded_wait() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let started = Instant::now();
        let error = read_frame(&mut reader, started + Duration::from_millis(50)).unwrap_err();
        assert!(matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(client);
    }

    #[test]
    fn trickling_clients_cannot_extend_the_request_deadline() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(address).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let writer = std::thread::spawn(move || {
            for _ in 0..10 {
                if client.write_all(b"M").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let mut reader = BufReader::new(stream);
        let started = Instant::now();
        let error = read_frame(&mut reader, started + Duration::from_millis(70)).unwrap_err();
        assert!(matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(reader);
        writer.join().unwrap();
    }
}
