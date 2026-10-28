const SIDEKICK_GUIDANCE: &str = "You are a session-scoped sidekick. The parent owns user intent, ambiguity, planning, decisions, and final review. Perform only the bounded mechanical inspection, edit, or test work delegated to you. Return a short evidence-backed report with relevant paths or results; do not dump raw tool logs. Escalate unresolved judgment or ambiguity to the parent.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidekickStatus {
    Idle,
    Busy,
    Stopped,
}

impl SidekickStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Busy => "busy",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidekickReport {
    pub spawn_id: u64,
    pub status: SidekickStatus,
    pub answer: String,
    pub identity: Option<SubagentIdentity>,
}

/// Immediate acknowledgement for a persistent sidekick task. The stable
/// `spawn_id` names the retained conversation for every later turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidekickAcknowledgement {
    pub spawn_id: u64,
    pub status: BackgroundStatus,
    pub identity: Option<SubagentIdentity>,
}

impl SidekickAcknowledgement {
    pub fn into_value(self) -> Value {
        json!({"spawnId": self.spawn_id, "status": self.status.as_str(),
            "identity": self.identity, "persistent": true, "termination": "detached"})
    }
}

impl SidekickReport {
    pub fn into_value(self) -> Value {
        json!({"spawnId": self.spawn_id, "status": self.status.as_str(), "answer": self.answer,
            "identity": self.identity, "termination": "retained"})
    }
}

pub(crate) struct SidekickRegistry<M: Model + Clone + 'static> {
    inner: Arc<SidekickRegistryInner<M>>,
}

struct SidekickRegistryInner<M: Model + Clone + 'static> {
    live: StdMutex<HashMap<u64, Arc<Sidekick<M>>>>,
    stopped: StdMutex<HashSet<u64>>,
    stopped_order: StdMutex<VecDeque<u64>>,
}

impl<M: Model + Clone + 'static> Clone for SidekickRegistry<M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<M: Model + Clone + 'static> Default for SidekickRegistry<M> {
    fn default() -> Self {
        Self {
            inner: Arc::new(SidekickRegistryInner {
                live: StdMutex::default(),
                stopped: StdMutex::default(),
                stopped_order: StdMutex::default(),
            }),
        }
    }
}

impl<M: Model + Clone + 'static> SidekickRegistryInner<M> {
    fn remember_stopped(&self, id: u64) {
        let mut stopped = self.stopped.lock().unwrap();
        if stopped.insert(id) {
            let mut order = self.stopped_order.lock().unwrap();
            order.push_back(id);
            if order.len() > 256 {
                if let Some(expired) = order.pop_front() {
                    stopped.remove(&expired);
                }
            }
        }
    }

    fn stop_all(&self) -> usize {
        let sidekicks = std::mem::take(&mut *self.live.lock().unwrap());
        let count = sidekicks.len();
        for (id, sidekick) in sidekicks {
            sidekick.stop();
            sidekick
                .manager
                .finish_sidekick(sidekick.generation, sidekick.spawn_id);
            self.remember_stopped(id);
        }
        count
    }
}

impl<M: Model + Clone + 'static> Drop for SidekickRegistryInner<M> {
    fn drop(&mut self) {
        self.stop_all();
    }
}

/// A sidekick admitted for one task: its handle, the task, and its slot.
type AdmittedSidekick<M> = (Arc<Sidekick<M>>, String, Option<background::manager::Slot>);

struct Sidekick<M: Model + Clone + 'static> {
    agent: Agent<M>,
    context: Mutex<Context>,
    identity: Option<SubagentIdentity>,
    lifecycle: StdMutex<SidekickLifecycle>,
    limits: Limits,
    timeout: u32,
    manager: Arc<background::manager::SubagentManagerInner>,
    generation: u64,
    session_cancellation: CancellationToken,
    spawn_id: u64,
    spawn: SubagentSpawn,
    notifier: background::SubagentNotifier,
    _in_flight: InFlight,
}

enum SidekickLifecycle {
    Idle,
    Busy(CancellationToken),
    Stopped,
}

impl<M: Model + Clone + 'static> Sidekick<M> {
    fn status(&self) -> SidekickStatus {
        if self.session_cancellation.is_cancelled() {
            return SidekickStatus::Stopped;
        }
        match &*self.lifecycle.lock().unwrap() {
            SidekickLifecycle::Idle => SidekickStatus::Idle,
            SidekickLifecycle::Busy(_) => SidekickStatus::Busy,
            SidekickLifecycle::Stopped => SidekickStatus::Stopped,
        }
    }

    fn begin(
        &self,
        parent: CancellationToken,
    ) -> Result<(CancellationToken, tokio::task::JoinHandle<()>), ToolError> {
        let mut lifecycle = self.lifecycle.lock().unwrap();
        if self.session_cancellation.is_cancelled() {
            *lifecycle = SidekickLifecycle::Stopped;
            return Err(ToolError::msg("sidekick is stopped"));
        }
        match &*lifecycle {
            SidekickLifecycle::Busy(_) => return Err(ToolError::msg("sidekick is busy")),
            SidekickLifecycle::Stopped => return Err(ToolError::msg("sidekick is stopped")),
            SidekickLifecycle::Idle => {}
        }
        let token = self.session_cancellation.child_token();
        let parent_token = token.clone();
        let watcher = tokio::spawn(async move {
            parent.cancelled().await;
            parent_token.cancel();
        });
        *lifecycle = SidekickLifecycle::Busy(token.clone());
        Ok((token, watcher))
    }

    fn finish(&self) {
        let mut lifecycle = self.lifecycle.lock().unwrap();
        if matches!(*lifecycle, SidekickLifecycle::Busy(_))
            && self.manager.sidekick_live(self.generation, self.spawn_id)
        {
            *lifecycle = SidekickLifecycle::Idle;
        }
        self.manager.sidekick_idle(self.generation, self.spawn_id);
    }

    fn stop(&self) {
        let mut lifecycle = self.lifecycle.lock().unwrap();
        if let SidekickLifecycle::Busy(token) = &*lifecycle {
            token.cancel();
        }
        *lifecycle = SidekickLifecycle::Stopped;
    }
}

struct TaskLease<M: Model + Clone + 'static> {
    sidekick: Arc<Sidekick<M>>,
    slot: Option<background::manager::Slot>,
    parent_watcher: tokio::task::JoinHandle<()>,
}

impl<M: Model + Clone + 'static> Drop for TaskLease<M> {
    fn drop(&mut self) {
        self.parent_watcher.abort();
        self.slot.take();
        self.sidekick.finish();
    }
}

struct InitialRegistration<M: Model + Clone + 'static> {
    registry: Arc<SidekickRegistryInner<M>>,
    sidekick: Arc<Sidekick<M>>,
    armed: bool,
}

impl<M: Model + Clone + 'static> Drop for InitialRegistration<M> {
    fn drop(&mut self) {
        if self.armed {
            self.registry
                .live
                .lock()
                .unwrap()
                .remove(&self.sidekick.spawn_id);
            self.sidekick.stop();
            self.sidekick
                .manager
                .finish_sidekick(self.sidekick.generation, self.sidekick.spawn_id);
        }
    }
}
