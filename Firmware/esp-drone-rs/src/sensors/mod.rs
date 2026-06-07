//! Sensor sampling tasks — producer side of the flight pipeline.
//!
//! Hardware access lives in [`crate::drivers`]; this layer owns sample rates, calibration,
//! and the shared state handed to [`crate::estimation`].

mod imu;
