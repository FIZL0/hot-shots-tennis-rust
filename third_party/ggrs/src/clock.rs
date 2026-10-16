//! hst patch (B494): ggrs' clock. Real time, unless the calling thread drives it (`drive`, `advance`): a test that
//! runs peers in rounds then gets the same pings, resends and time sync however long a round really takes.

use instant::Duration;
pub use instant::Instant;
use std::cell::Cell;

thread_local! {
    /// The driven clock: its start (a real instant) and the time advanced since.
    static DRIVEN: Cell<Option<(Instant, Duration)>> = const { Cell::new(None) };
}

/// Now, by this thread's clock.
pub fn now() -> Instant {
    DRIVEN.with(|d| d.get()).map_or_else(Instant::now, |(t0, d)| t0 + d)
}

/// Milliseconds since the Unix epoch (pings), by this thread's clock; driven, since the drive began.
pub(crate) fn millis_since_epoch() -> u128 {
    if let Some((_, d)) = DRIVEN.with(|d| d.get()) {
        return d.as_millis();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::new_0().get_time() as u128
    }
}

/// From now on this thread's clock stands still but for `advance` (`false`: real time again).
pub fn drive(on: bool) {
    DRIVEN.with(|d| d.set(on.then(|| (Instant::now(), Duration::ZERO))));
}

/// Moves this thread's driven clock on by `by` (nothing if it isn't driven).
pub fn advance(by: Duration) {
    DRIVEN.with(|d| d.set(d.get().map(|(t0, t)| (t0, t + by))));
}
