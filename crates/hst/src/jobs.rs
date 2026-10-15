//! B409: work handed off so a frame stays short (a mode's start: its menu sheets, the match's characters). On native
//! it runs on the async pool's threads (P50). In the browser that pool is the page's only thread, and it ran every
//! spawned task as a microtask right after the frame that spawned it, all back to back: a match start's sheets and
//! characters froze the page 1-2 s on desktop and several seconds on a phone. There a job waits in a queue instead
//! and [`run`] does them between frames, a frame's budget at a time; a long one ([`spawn_steps`]) a step at a time.

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex, Weak};

/// A job handed off by [`spawn`], with its result once it's done.
pub struct Job<T>(Inner<T>);

#[cfg(not(target_arch = "wasm32"))]
type Inner<T> = bevy::tasks::Task<T>;
#[cfg(target_arch = "wasm32")]
type Inner<T> = Arc<Mutex<Slot<T>>>;

/// Hands `f` off: on the async pool, or (browser) to the queue [`run`] works through. None without an async pool
/// (headless tests): the caller does the work itself.
pub fn spawn<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<Job<T>> {
    let mut f = Some(f);
    spawn_steps(move || f.take().map(|f| f()))
}

/// [`spawn`] for a job done in steps: `step` is called until it gives the result. In the browser each step is a
/// piece of a frame's budget, so it should be short (a few ms on desktop); on native they all run in one go.
pub fn spawn_steps<T: Send + 'static>(mut step: impl FnMut() -> Option<T> + Send + 'static) -> Option<Job<T>> {
    let pool = bevy::tasks::AsyncComputeTaskPool::try_get()?;
    #[cfg(not(target_arch = "wasm32"))]
    return Some(Job(pool.spawn(async move {
        loop {
            if let Some(v) = step() {
                break v;
            }
        }
    })));
    #[cfg(target_arch = "wasm32")]
    {
        let _ = pool;
        let slot = Arc::new(Mutex::new(Slot::Todo(Box::new(step))));
        let weak: Weak<dyn Pending> = Arc::downgrade(&slot) as Weak<Mutex<Slot<T>>>;
        QUEUE.lock().unwrap().push_back(weak);
        Some(Job(slot))
    }
}

impl<T: Send + 'static> Job<T> {
    /// Whether it's done.
    pub fn is_finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.0.is_finished();
        #[cfg(target_arch = "wasm32")]
        return !matches!(*self.0.lock().unwrap(), Slot::Todo(_));
    }

    /// Its result if it's done (once).
    pub fn poll(&mut self) -> Option<T> {
        #[cfg(not(target_arch = "wasm32"))]
        return bevy::tasks::block_on(bevy::tasks::poll_once(&mut self.0));
        #[cfg(target_arch = "wasm32")]
        {
            let mut s = self.0.lock().unwrap();
            if !matches!(*s, Slot::Done(_)) {
                return None;
            }
            let Slot::Done(v) = std::mem::replace(&mut *s, Slot::Taken) else { unreachable!() };
            Some(v)
        }
    }

    /// Its result: waits for it (native), or does what's left of it now (browser).
    pub fn wait(self) -> T {
        #[cfg(not(target_arch = "wasm32"))]
        return bevy::tasks::block_on(self.0);
        #[cfg(target_arch = "wasm32")]
        {
            while self.0.step() {}
            let mut job = self;
            job.poll().expect("a job taken twice")
        }
    }
}

#[cfg(target_arch = "wasm32")]
enum Slot<T> {
    Todo(Box<dyn FnMut() -> Option<T> + Send>),
    Done(T),
    Taken,
}

/// A queued job, whatever its result.
#[cfg(target_arch = "wasm32")]
trait Pending: Send + Sync {
    /// Does its next step; true while there's more.
    fn step(&self) -> bool;
}

#[cfg(target_arch = "wasm32")]
impl<T: Send> Pending for Mutex<Slot<T>> {
    fn step(&self) -> bool {
        let mut s = self.lock().unwrap();
        let Slot::Todo(f) = &mut *s else { return false };
        let Some(v) = f() else { return true };
        *s = Slot::Done(v);
        false
    }
}

/// Queued jobs in the order they were handed off (gone once their [`Job`] is dropped: nobody wants the result).
#[cfg(target_arch = "wasm32")]
static QUEUE: Mutex<std::collections::VecDeque<Weak<dyn Pending>>> = Mutex::new(std::collections::VecDeque::new());

/// A frame's budget for queued jobs (ms): one step is always done, a step is never cut, so a long one runs over.
#[cfg(target_arch = "wasm32")]
const BUDGET: std::time::Duration = std::time::Duration::from_millis(25);

/// Browser: does queued jobs' steps, oldest job first, until this frame's [`BUDGET`] is used. Nothing on native.
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        let until = bevy::platform::time::Instant::now() + BUDGET;
        loop {
            let Some(next) = QUEUE.lock().unwrap().pop_front() else { return };
            let Some(job) = next.upgrade() else { continue };
            if job.step() {
                QUEUE.lock().unwrap().push_front(next);
            }
            if bevy::platform::time::Instant::now() >= until {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A job done in steps gives the same result as in one go, whether waited for or polled until it lands.
    #[test]
    fn steps_run_to_the_result() {
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let counter = || {
            let mut n = 0;
            move || {
                n += 1;
                (n == 5).then_some(n * 10)
            }
        };
        assert_eq!(spawn_steps(counter()).unwrap().wait(), 50);
        let mut job = spawn_steps(counter()).unwrap();
        let got = loop {
            run();
            if let Some(v) = job.poll() {
                break v;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        assert_eq!(got, 50);
        assert_eq!(spawn(|| 7).unwrap().wait(), 7);
    }
}
