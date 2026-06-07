//! Cross-cutting safety — arming gate, watchdog, link-loss failsafe.
//!
//! ```text
//! safety/
//!   mod.rs     — this file
//!   arming.rs  — disarmed-by-default; explicit arm/disarm semantics
//! ```
//!
//! All motor output paths should pass through the arming gate before reaching [`crate::drivers::motors::Motors`].

mod arming;
