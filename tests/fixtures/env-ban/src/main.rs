//! Planted process-global environment mutation. `unsafe` is required in
//! edition 2024; without it this fails to compile (E0133) instead of tripping
//! the lint the test is about.

fn main() {
    // SAFETY: single-threaded fixture; it exists only to be linted, never run.
    unsafe { std::env::set_var("WSX_ENV_BAN_FIXTURE", "1") };
    // SAFETY: as above.
    unsafe { std::env::remove_var("WSX_ENV_BAN_FIXTURE") };
}
