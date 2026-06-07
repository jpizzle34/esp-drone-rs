//! 1 kHz stabilizer task — sensor read → estimate → control → mix → motors.
//!
//! Runs on the application CPU; must not block or allocate on the hot path.
