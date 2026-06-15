//! Arming state — disarmed by default; explicit arm required for motor output.

use std::sync::atomic::{AtomicBool, Ordering};

/// Why the vehicle left the armed state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisarmReason {
    Command,
    Stop,
    JoyQuit,
}

pub struct Arming {
    armed: AtomicBool,
}

impl Arming {
    pub fn new() -> Self {
        Self {
            armed: AtomicBool::new(false),
        }
    }

    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }

    pub fn arm(&self) {
        self.armed.store(true, Ordering::Release);
        log::info!("armed — motor output enabled");
    }

    pub fn disarm(&self, reason: DisarmReason) {
        if self.armed.swap(false, Ordering::AcqRel) {
            log::info!("disarmed ({reason:?})");
        }
    }
}
