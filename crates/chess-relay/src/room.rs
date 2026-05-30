//! In-memory room registry.
//!
//! A room exists only between "host created it" and "guest paired with it".
//! Room numbers are recycled: once a guest joins (or the host leaves) the entry
//! is removed and the number can be reused by a future room. Uniqueness is only
//! guaranteed across *currently active* rooms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;

use crate::WsServer;

/// What a joining guest hands off to the waiting host task: its WebSocket
/// stream so the host task can relay between both peers.
pub type GuestStream = WsServer;

/// A room awaiting a guest.
struct RoomEntry {
    /// The host's base64 salt, echoed to the guest on join.
    salt: String,
    /// One-shot channel used to hand the guest's stream to the host task.
    tx: oneshot::Sender<GuestStream>,
}

/// Shared registry of waiting rooms.
#[derive(Clone, Default)]
pub struct Rooms {
    inner: Arc<Mutex<HashMap<String, RoomEntry>>>,
}

impl Rooms {
    /// Register a new room with a unique 8-digit number, returning the number
    /// and the receiver the host task awaits the guest stream on.
    pub fn create(&self, salt: String) -> (String, oneshot::Receiver<GuestStream>) {
        let (tx, rx) = oneshot::channel();
        let mut map = self.inner.lock().expect("rooms mutex poisoned");
        let room = loop {
            let candidate = random_room_number();
            if !map.contains_key(&candidate) {
                break candidate;
            }
        };
        map.insert(room.clone(), RoomEntry { salt, tx });
        (room, rx)
    }

    /// Take ("claim") a waiting room by number, removing it from the registry.
    /// Returns the host's salt and the sender used to hand off the guest stream.
    pub fn take(&self, room: &str) -> Option<(String, oneshot::Sender<GuestStream>)> {
        self.inner
            .lock()
            .expect("rooms mutex poisoned")
            .remove(room)
            .map(|e| (e.salt, e.tx))
    }

    /// Remove a room if it is still waiting (e.g. host timed out or left).
    pub fn remove(&self, room: &str) {
        self.inner
            .lock()
            .expect("rooms mutex poisoned")
            .remove(room);
    }

    /// Number of currently active (waiting) rooms.
    pub fn active(&self) -> usize {
        self.inner.lock().expect("rooms mutex poisoned").len()
    }
}

/// Generate a random 8-digit room number (`00000000`–`99999999`).
fn random_room_number() -> String {
    let mut buf = [0u8; 4];
    getrandom::fill(&mut buf).expect("system RNG must be available");
    let n = u32::from_le_bytes(buf) % 100_000_000;
    format!("{n:08}")
}
