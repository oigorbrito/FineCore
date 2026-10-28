use super::super::{SubagentDepth, SubagentSpawn};
use serde_json::Value;
use std::sync::{atomic::AtomicU64, Arc};

mod inner;
#[cfg(test)]
mod tests;

/// No default concurrency limit. Zero means unrestricted; an explicit positive
/// limit queues excess workers in spawn order and can be changed live.
pub const DEFAULT_BACKGROUND_SUBAGENT_LIMIT: u32 = 0;

/// Terminal result of one detached subagent, delivered to the interactive
/// host after the original tool call has already returned its acknowledgement.
#[derive(Clone, Debug)]
pub struct SubagentNotification {
    pub generation: u64,
    pub spawn: SubagentSpawn,
    pub result: Result<Value, String>,
}

/// Where an admitted background job is in its lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundStatus {
    /// Admitted and waiting, in spawn order, for a running slot.
    Queued,
    /// Holds a slot and is executing model or tool steps.
    Running,
}

impl BackgroundStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
        }
    }
}

/// One admitted background job as `action=list` reports it.
#[derive(Clone, Debug)]
pub struct BackgroundJob {
    pub spawn: SubagentSpawn,
    pub status: BackgroundStatus,
}

struct SubagentManagerState {
    generation: u64,
    cancellation: orca_harness_core::CancellationToken,
    jobs: std::collections::HashMap<u64, ActiveSubagent>,
    /// Admitted spawns still waiting for a slot, oldest first.
    queue: std::collections::VecDeque<u64>,
    /// Live slot guards across all notification generations. Cancellation
    /// requests stop work, but capacity is released only when workers exit.
    running: usize,
    completion_capacity: Option<usize>,
    /// Reservations survive worker completion until the host consumes it.
    /// Completion reservations by spawn. Sidekicks reuse one spawn id, so
    /// each completed turn needs its own reservation rather than set-style
    /// deduplication.
    unacknowledged: std::collections::HashMap<u64, usize>,
    /// Ordinary jobs cancelled without ending the session. Their late
    /// notifications stay host-observable but must not enter the parent.
    suppressed: std::collections::HashSet<u64>,
    /// Set by [`SubagentManager::close`]: every later admission is refused.
    closed: bool,
}

struct ActiveSubagent {
    spawn: SubagentSpawn,
    cancellation: orca_harness_core::CancellationToken,
    status: BackgroundStatus,
    on_cancel: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    peak_running: usize,
    running_children: usize,
    /// Retained sidekicks share manager admission and lifetime but are not
    /// ordinary detached completions exposed by `active`.
    sidekick: bool,
    sidekick_active: bool,
}

pub(crate) struct SubagentManagerInner {
    state: std::sync::Mutex<SubagentManagerState>,
    /// Source of the live concurrency limit. Held here so its watch stays
    /// open for as long as any job may wait on it.
    settings: SubagentDepth,
    /// Bumped whenever a slot frees or the queue head changes; every
    /// waiter re-checks whether it is next.
    slots: tokio::sync::watch::Sender<u64>,
    pub(super) spawn_seq: Arc<AtomicU64>,
}

impl Drop for SubagentManagerInner {
    fn drop(&mut self) {
        self.state.get_mut().unwrap().cancellation.cancel();
    }
}

/// Session-owned lifetime and concurrency boundary for detached subagents.
#[derive(Clone)]
pub struct SubagentManager {
    pub(crate) inner: Arc<SubagentManagerInner>,
    _lifetime: Option<Arc<ManagerLifetime>>,
}

struct ManagerLifetime(std::sync::Weak<SubagentManagerInner>);

impl Drop for ManagerLifetime {
    fn drop(&mut self) {
        if let Some(inner) = self.0.upgrade() {
            inner.reset();
        }
    }
}

impl Default for SubagentManager {
    fn default() -> Self {
        Self::new(DEFAULT_BACKGROUND_SUBAGENT_LIMIT)
    }
}

/// A running slot. Dropping it frees the slot and wakes the queue, however
/// the job ends.
pub(crate) struct Slot(Arc<SubagentManagerInner>, Option<u64>);

/// Rolls registration back if host extension setup fails before detachment.
pub(crate) struct Admission {
    manager: Arc<SubagentManagerInner>,
    generation: u64,
    spawn_id: u64,
    cancellation: orca_harness_core::CancellationToken,
    slot: Option<Slot>,
    transferred: bool,
}

impl Admission {
    pub(crate) fn into_parts(
        mut self,
    ) -> (u64, orca_harness_core::CancellationToken, Option<Slot>) {
        self.transferred = true;
        (self.generation, self.cancellation.clone(), self.slot.take())
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        if self.transferred {
            return;
        }
        let mut state = self.manager.state.lock().unwrap();
        if state.generation == self.generation {
            state.jobs.remove(&self.spawn_id);
            state.queue.retain(|id| *id != self.spawn_id);
            state.unacknowledged.remove(&self.spawn_id);
        }
        drop(state);
        self.manager.wake();
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.running = state.running.saturating_sub(1);
        if let Some(root) = self.1.and_then(|run| state.jobs.get_mut(&run)) {
            root.running_children = root.running_children.saturating_sub(1);
        }
        drop(state);
        self.0.wake();
    }
}

impl SubagentManager {
    /// A manager with a fixed limit, for hosts without a `/subagents`
    /// settings handle.
    pub fn new(max_concurrent: u32) -> Self {
        let settings = SubagentDepth::default();
        settings.set_background_limit(max_concurrent);
        Self::from_settings(settings)
    }

    /// A manager whose limit follows `settings.background_limit()` live:
    /// raising it admits queued jobs at once.
    pub fn from_settings(settings: SubagentDepth) -> Self {
        let inner = Arc::new(SubagentManagerInner {
            state: std::sync::Mutex::new(SubagentManagerState {
                generation: 0,
                cancellation: orca_harness_core::CancellationToken::new(),
                jobs: std::collections::HashMap::new(),
                queue: std::collections::VecDeque::new(),
                running: 0,
                completion_capacity: None,
                unacknowledged: std::collections::HashMap::new(),
                suppressed: std::collections::HashSet::new(),
                closed: false,
            }),
            settings,
            slots: tokio::sync::watch::Sender::new(0),
            spawn_seq: Arc::new(AtomicU64::new(0)),
        });
        Self {
            _lifetime: Some(Arc::new(ManagerLifetime(Arc::downgrade(&inner)))),
            inner,
        }
    }

    pub(crate) fn spawn_sequence(&self) -> Arc<AtomicU64> {
        self.inner.spawn_seq.clone()
    }

    /// Bound admitted jobs plus completed results awaiting host consumption.
    /// Configure before admitting jobs. Zero rejects every background spawn.
    pub fn with_completion_capacity(self, capacity: usize) -> Self {
        let mut state = self.inner.state.lock().unwrap();
        assert!(state.jobs.is_empty() && state.unacknowledged.is_empty());
        state.completion_capacity = Some(capacity);
        drop(state);
        self
    }

    /// Release a completion reservation after its result has been consumed.
    pub fn acknowledge(&self, generation: u64, spawn_id: u64) {
        let mut state = self.inner.state.lock().unwrap();
        if state.generation == generation
            || state.jobs.get(&spawn_id).is_some_and(|job| job.sidekick)
        {
            if let Some(count) = state.unacknowledged.get_mut(&spawn_id) {
                *count -= 1;
                if *count == 0 {
                    state.unacknowledged.remove(&spawn_id);
                }
            }
        }
    }

    pub(crate) fn worker_handle(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _lifetime: None,
        }
    }

    pub fn cancel_all(&self) -> usize {
        self.inner.cancel_all()
    }
    /// Replace the current session generation, cancelling ordinary workers
    /// and retained sidekicks. Session lifecycle owners use this; the model
    /// tool's `cancel_all` intentionally affects ordinary workers only.
    pub fn reset(&self) -> usize {
        self.inner.reset()
    }
    /// Stop admitting work for good, then cancel every admitted job as
    /// [`cancel_all`](Self::cancel_all) does. Every admission path (host
    /// spawns, the model tool, workflow stages) is refused afterwards, so
    /// [`wait_idle`](Self::wait_idle) cannot be extended by new work.
    pub fn close(&self) -> usize {
        self.inner.close()
    }
    pub fn cancel(&self, spawn_id: u64) -> bool {
        self.inner.cancel(spawn_id)
    }
    pub fn active(&self) -> Vec<BackgroundJob> {
        self.inner.active()
    }
    /// Automatic parent context excludes workflow runs and their isolated stages.
    /// Explicit inventories and the UI may still use `active`.
    pub fn active_for_parent(&self) -> Vec<BackgroundJob> {
        self.inner.active_visible(false)
    }

    pub fn is_current(&self, generation: u64) -> bool {
        self.inner.is_current(generation)
    }

    pub(crate) fn accepts_notification(&self, generation: u64, spawn_id: u64) -> bool {
        let state = self.inner.state.lock().unwrap();
        (state.generation == generation
            || state.jobs.get(&spawn_id).is_some_and(|job| job.sidekick))
            && !state.suppressed.contains(&spawn_id)
    }

    /// Workers still holding a slot or an admitted job: every job in the
    /// current generation, plus workers of earlier generations that were
    /// cancelled but still hold a running slot. Cancellation is
    /// cooperative, so this stays positive until those workers observe
    /// their token and return.
    pub fn live_workers(&self) -> usize {
        self.inner.live_workers()
    }

    /// Resolve once [`live_workers`](Self::live_workers) reaches zero.
    /// Driven by the same watch that frees slots and finishes jobs, so
    /// there is no polling; a manager that is already idle resolves at
    /// once. Bound the wait with a timeout when workers may ignore
    /// cancellation. A worker releases its slot before its exit
    /// notification is sent, so that notification may still be in
    /// flight when this returns.
    pub async fn wait_idle(&self) {
        let mut slots = self.inner.slots.subscribe();
        loop {
            // Mark the current epoch seen before checking, so a wake that
            // lands between the check and the wait is not missed.
            let _ = slots.borrow_and_update();
            if self.inner.live_workers() == 0 {
                return;
            }
            if slots.changed().await.is_err() {
                return;
            }
        }
    }
}
