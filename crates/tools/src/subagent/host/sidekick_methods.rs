impl<M: Model + Clone + 'static> SubagentTool<M> {
    fn prepare_sidekick(
        &self,
        request: SubagentRequest,
        completion: bool,
    ) -> Result<AdmittedSidekick<M>, ToolError> {
        if self.background.is_none() {
            return Err(ToolError::msg(
                "persistent sidekicks require session-owned background subagents",
            ));
        }
        let prepared = self.prepare_request(request, None, false, None, true)?;
        let Prepared {
            agent,
            spawn,
            mut context,
            in_flight,
            mut limits,
            timeout,
            ..
        } = prepared;
        let id = spawn.id;
        let task = spawn.task.clone();
        context.push_system(SIDEKICK_GUIDANCE);
        // `prepare_spawn` intentionally did not admit a foreground run.
        // Register only after all fallible preparation/extensions complete,
        // atomically checking the manager's open generation and capacity.
        // Retain a reusable limits template: each task derives a new deadline.
        limits.deadline = self.limits.deadline;
        let manager = self.background.as_ref().unwrap().manager.inner.clone();
        let notifier = self.background.as_ref().unwrap().notifier.clone();
        let admission = manager
            .admit_sidekick(&spawn, completion)
            .map_err(ToolError::msg)?;
        let (generation, session_cancellation, slot) = admission.into_parts();
        let sidekick = Arc::new(Sidekick {
            agent,
            context: Mutex::new(context),
            identity: spawn.identity.clone(),
            lifecycle: StdMutex::new(SidekickLifecycle::Idle),
            limits,
            timeout,
            manager,
            generation,
            session_cancellation,
            spawn_id: id,
            spawn,
            notifier,
            _in_flight: in_flight,
        });
        self.sidekicks
            .inner
            .live
            .lock()
            .unwrap()
            .insert(id, sidekick.clone());
        Ok((sidekick, task, slot))
    }

    /// Start a persistent sidekick in the background and return immediately.
    pub fn start_sidekick(
        &self,
        request: SubagentRequest,
    ) -> Result<SidekickAcknowledgement, ToolError> {
        let (sidekick, task, slot) = self.prepare_sidekick(request, true)?;
        let status = if slot.is_some() {
            BackgroundStatus::Running
        } else {
            BackgroundStatus::Queued
        };
        let acknowledgement = SidekickAcknowledgement {
            spawn_id: sidekick.spawn_id,
            status,
            identity: sidekick.identity.clone(),
        };
        self.spawn_sidekick_task(sidekick, task, slot);
        Ok(acknowledgement)
    }

    /// Explicit foreground form retained for callers that need to await the
    /// initial report. A failed first turn removes its undisclosed handle.
    pub async fn start_sidekick_foreground(
        &self,
        request: SubagentRequest,
        cancellation: CancellationToken,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<SidekickReport, ToolError> {
        let (sidekick, task, slot) = self.prepare_sidekick(request, false)?;
        let mut registration = InitialRegistration {
            registry: self.sidekicks.inner.clone(),
            sidekick: sidekick.clone(),
            armed: true,
        };
        let (token, parent_watcher) = sidekick.begin(cancellation)?;
        let slot = match slot {
            Some(slot) => Some(slot),
            None => Some(
                sidekick
                    .manager
                    .acquire(sidekick.spawn_id, &token)
                    .await
                    .ok_or_else(|| ToolError::msg("sidekick is stopped"))?,
            ),
        };
        let lease = TaskLease {
            sidekick: sidekick.clone(),
            slot,
            parent_watcher,
        };
        let result = self
            .run_sidekick_task(sidekick.clone(), &task, token, deadline, lease)
            .await;
        if result.is_ok() {
            registration.armed = false;
        }
        result
    }

    /// Queue one background turn on an idle sidekick and return immediately.
    pub fn sidekick_task(
        &self,
        spawn_id: u64,
        task: &str,
    ) -> Result<SidekickAcknowledgement, ToolError> {
        let sidekick = self
            .sidekicks
            .inner
            .live
            .lock()
            .unwrap()
            .get(&spawn_id)
            .cloned()
            .ok_or_else(|| self.sidekick_lookup_error(spawn_id))?;
        if !sidekick
            .manager
            .sidekick_live(sidekick.generation, sidekick.spawn_id)
        {
            sidekick.stop();
            return Err(ToolError::msg("sidekick is stopped"));
        }
        // Claim Busy before admission so overlapping calls reject even while
        // this turn is queued behind the manager's bounded concurrency.
        let (token, parent_watcher) = sidekick.begin(CancellationToken::new())?;
        let slot = match sidekick
            .manager
            .queue_sidekick(sidekick.generation, spawn_id, true)
        {
            Ok(slot) => slot,
            Err(error) => {
                parent_watcher.abort();
                sidekick.finish();
                return Err(ToolError::msg(error));
            }
        };
        let status = if slot.is_some() {
            BackgroundStatus::Running
        } else {
            BackgroundStatus::Queued
        };
        let acknowledgement = SidekickAcknowledgement {
            spawn_id,
            status,
            identity: sidekick.identity.clone(),
        };
        self.spawn_admitted_sidekick_task(sidekick, task.to_string(), token, parent_watcher, slot);
        Ok(acknowledgement)
    }

    /// Explicit foreground follow-up.
    pub async fn sidekick_task_foreground(
        &self,
        spawn_id: u64,
        task: &str,
        cancellation: CancellationToken,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<SidekickReport, ToolError> {
        let sidekick = self
            .sidekicks
            .inner
            .live
            .lock()
            .unwrap()
            .get(&spawn_id)
            .cloned()
            .ok_or_else(|| self.sidekick_lookup_error(spawn_id))?;
        let (token, parent_watcher) = sidekick.begin(cancellation)?;
        let mut lease = TaskLease {
            sidekick: sidekick.clone(),
            slot: None,
            parent_watcher,
        };
        let slot = match sidekick
            .manager
            .queue_sidekick(sidekick.generation, spawn_id, false)
        {
            Ok(Some(slot)) => slot,
            Ok(None) => sidekick
                .manager
                .acquire(spawn_id, &token)
                .await
                .ok_or_else(|| ToolError::msg("sidekick is stopped"))?,
            Err(error) => return Err(ToolError::msg(error)),
        };
        lease.slot = Some(slot);
        self.run_sidekick_task(sidekick, task, token, deadline, lease)
            .await
    }

    fn spawn_sidekick_task(
        &self,
        sidekick: Arc<Sidekick<M>>,
        task: String,
        slot: Option<background::manager::Slot>,
    ) {
        let (token, parent_watcher) = sidekick
            .begin(CancellationToken::new())
            .expect("new sidekick is idle");
        self.spawn_admitted_sidekick_task(sidekick, task, token, parent_watcher, slot);
    }

    fn spawn_admitted_sidekick_task(
        &self,
        sidekick: Arc<Sidekick<M>>,
        task: String,
        token: CancellationToken,
        parent_watcher: tokio::task::JoinHandle<()>,
        slot: Option<background::manager::Slot>,
    ) {
        let tool = self.clone();
        tokio::spawn(async move {
            let slot = match slot {
                Some(slot) => Some(slot),
                None => sidekick.manager.acquire(sidekick.spawn_id, &token).await,
            };
            let result = match slot {
                Some(slot) => {
                    let lease = TaskLease {
                        sidekick: sidekick.clone(),
                        slot: Some(slot),
                        parent_watcher,
                    };
                    tool.run_sidekick_task(sidekick.clone(), &task, token, None, lease)
                        .await
                        .map(SidekickReport::into_value)
                        .map_err(|error| error.to_string())
                }
                None => {
                    parent_watcher.abort();
                    sidekick.finish();
                    Err("sidekick stopped before execution".to_string())
                }
            };
            if sidekick
                .manager
                .sidekick_live(sidekick.generation, sidekick.spawn_id)
            {
                let mut spawn = sidekick.spawn.clone();
                spawn.task = task;
                (sidekick.notifier)(SubagentNotification {
                    generation: sidekick.manager.generation(),
                    spawn,
                    result,
                });
            }
        });
    }

    async fn run_sidekick_task(
        &self,
        sidekick: Arc<Sidekick<M>>,
        task: &str,
        token: CancellationToken,
        deadline: Option<tokio::time::Instant>,
        _lease: TaskLease<M>,
    ) -> Result<SidekickReport, ToolError> {
        let spawn_id = sidekick.spawn_id;
        // Execute transactionally. Cancellation, errors, and dropping this
        // future discard the working transcript, including unmatched tool
        // calls; only a complete final response replaces retained context.
        let mut working = sidekick.context.lock().await.clone();
        working.push_user(task);
        let mut limits = sidekick.limits.clone();
        limits.deadline = execution_deadline(
            limits.deadline.into_iter().chain(deadline).min(),
            sidekick.timeout,
        );
        // `Agent` deliberately exposes no mutable per-run limits API. Its
        // retained sidekick instance therefore has no deadline; enforce this
        // task's derived deadline through its task cancellation token.
        if let Some(deadline) = limits.deadline {
            let timeout = token.clone();
            tokio::spawn(async move {
                tokio::time::sleep_until(deadline).await;
                timeout.cancel();
            });
        }
        let result = sidekick.agent.run_context(&mut working, token).await;
        let answer =
            result.map_err(|error| ToolError::msg(format!("sidekick task failed: {error}")))?;
        *sidekick.context.lock().await = working;
        Ok(SidekickReport {
            spawn_id,
            status: SidekickStatus::Idle,
            answer,
            identity: sidekick.identity.clone(),
        })
    }

    pub fn sidekick_status(&self, spawn_id: u64) -> Result<SidekickStatus, ToolError> {
        if let Some(sidekick) = self
            .sidekicks
            .inner
            .live
            .lock()
            .unwrap()
            .get(&spawn_id)
            .cloned()
        {
            if !sidekick
                .manager
                .sidekick_live(sidekick.generation, sidekick.spawn_id)
            {
                sidekick.stop();
            }
            return Ok(sidekick.status());
        }
        if self
            .sidekicks
            .inner
            .stopped
            .lock()
            .unwrap()
            .contains(&spawn_id)
        {
            Ok(SidekickStatus::Stopped)
        } else {
            Err(ToolError::msg("unknown sidekick handle"))
        }
    }

    pub fn stop_sidekick(&self, spawn_id: u64) -> Result<(), ToolError> {
        if self
            .sidekicks
            .inner
            .stopped
            .lock()
            .unwrap()
            .contains(&spawn_id)
        {
            return Ok(());
        }
        let sidekick = self
            .sidekicks
            .inner
            .live
            .lock()
            .unwrap()
            .remove(&spawn_id)
            .ok_or_else(|| ToolError::msg("unknown sidekick handle"))?;
        sidekick.stop();
        sidekick
            .manager
            .finish_sidekick(sidekick.generation, spawn_id);
        self.sidekicks.inner.remember_stopped(spawn_id);
        Ok(())
    }

    pub fn stop_all_sidekicks(&self) -> usize {
        self.sidekicks.inner.stop_all()
    }

    /// Move `previous`'s live and stopped sidekick handles onto this tool.
    /// A host that rebuilds its parent agent (model switch, reload) calls
    /// this so sidekicks survive the rebuild instead of stopping when the
    /// old tool drops. Each sidekick owns its own agent and context, so it
    /// keeps the configuration it was spawned with.
    pub fn adopt_sidekicks(&self, previous: &Self) {
        if Arc::ptr_eq(&self.sidekicks.inner, &previous.sidekicks.inner) {
            return;
        }
        let live = std::mem::take(&mut *previous.sidekicks.inner.live.lock().unwrap());
        self.sidekicks.inner.live.lock().unwrap().extend(live);
        let stopped = std::mem::take(&mut *previous.sidekicks.inner.stopped_order.lock().unwrap());
        previous.sidekicks.inner.stopped.lock().unwrap().clear();
        for id in stopped {
            self.sidekicks.inner.remember_stopped(id);
        }
    }

    fn sidekick_lookup_error(&self, spawn_id: u64) -> ToolError {
        if self
            .sidekicks
            .inner
            .stopped
            .lock()
            .unwrap()
            .contains(&spawn_id)
        {
            ToolError::msg("sidekick is stopped")
        } else {
            ToolError::msg("unknown sidekick handle")
        }
    }

}
