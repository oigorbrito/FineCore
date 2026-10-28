/// The worker's current endpoint: which provider, where, with what key,
/// and which model id is active. Type-erasing the adapter behind
/// `Arc<dyn Model>` is what lets one session switch providers.
#[derive(Clone)]
struct Endpoint {
    provider: Provider,
    base_url: String,
    api_key: Option<String>,
    model: String,
    reasoning_effort: Option<String>,
    max_output_tokens: Option<u64>,
    prompt_cache: bool,
    request_session_id: Option<String>,
    model_retries: Arc<AtomicU64>,
    model_gates: model_gates::ModelGates,
    subagent_settings: orca_harness_tools::SubagentDepth,
}

impl Endpoint {
    fn from_config(cfg: &Config) -> Self {
        let subagent_settings = subagent_settings::configured(
            cfg.subagent_depth,
            std::env::var("ORCA_MODEL_CONCURRENCY")
                .ok()
                .and_then(|value| value.parse().ok()),
        );
        Self {
            provider: cfg.provider,
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
            model: cfg.model.clone(),
            reasoning_effort: cfg.reasoning_effort.clone(),
            max_output_tokens: cfg.max_output_tokens,
            prompt_cache: cfg.prompt_cache,
            request_session_id: cfg
                .prompt_cache
                .then(orca_harness_extensions::new_session_id),
            model_retries: Arc::new(AtomicU64::new(0)),
            model_gates: Default::default(),
            subagent_settings,
        }
    }

    async fn list_models(
        &self,
    ) -> Result<Vec<openrouter::ModelInfo>, orca_harness_core::ModelError> {
        match self.provider {
            Provider::Anthropic => {
                orca_harness_model_providers::anthropic::list_models(
                    &self.base_url,
                    self.api_key.as_deref(),
                )
                .await
            }
            Provider::OpenAiCodex => {
                orca_harness_model_providers::openai_codex::list_models(Arc::new(
                    auth::CodexCliCredential::discover(),
                ))
                .await
            }
            Provider::Vercel => {
                orca_harness_model_providers::vercel::list_models(self.api_key.as_deref()).await
            }
            Provider::CheaperInference => {
                orca_harness_model_providers::cheaperinference::list_models().await
            }
            Provider::OpenRouter | Provider::OpenAi | Provider::Local => {
                openrouter::list_models(&self.base_url, self.api_key.as_deref()).await
            }
        }
    }

    /// Keep a requested or saved model only while the provider still advertises
    /// it. Otherwise use the first catalog entry, whose ordering is the
    /// provider adapter's default-selection policy.
    async fn catalog_model(&self, requested: Option<&str>) -> Result<String, String> {
        let models = self
            .list_models()
            .await
            .map_err(|error| error.to_string())?;
        if let Some(requested) = requested.filter(|model| !model.is_empty()) {
            if models.iter().any(|model| model.id == requested) {
                return Ok(requested.to_string());
            }
        }
        models
            .first()
            .map(|model| model.id.clone())
            .ok_or_else(|| format!("{} returned an empty model catalog", self.provider.label()))
    }

    fn build_model(&self) -> Arc<dyn Model> {
        self.build_model_for_ui_with_label(None, None)
    }

    fn model_retry_counter(&self) -> Arc<AtomicU64> {
        self.model_retries.clone()
    }

    fn build_model_for_ui(&self, ui: Option<mpsc::UnboundedSender<UiMsg>>) -> Arc<dyn Model> {
        self.build_model_for_ui_with_label(ui, None)
    }

    fn build_model_for_ui_with_label(
        &self,
        ui: Option<mpsc::UnboundedSender<UiMsg>>,
        retry_label: Option<String>,
    ) -> Arc<dyn Model> {
        let model: Arc<dyn Model> = match self.provider {
            Provider::Anthropic => {
                let mut model = orca_harness_model_providers::AnthropicModel::new(&self.model)
                    .base_url(&self.base_url)
                    .prompt_cache(self.prompt_cache);
                if let Some(key) = &self.api_key {
                    model = model.api_key(key);
                }
                if let Some(max_tokens) = self.max_output_tokens {
                    model = model.max_tokens(max_tokens);
                }
                if let Some(effort) = &self.reasoning_effort {
                    model = model.reasoning_effort(effort);
                }
                Arc::new(model)
            }
            Provider::OpenRouter => {
                let mut model = OpenRouterModel::new(self.model.as_str())
                    .base_url(self.base_url.clone())
                    .user_agent(ORCACODE_USER_AGENT)
                    .referer(ORCACODE_REFERER)
                    .title("Orca Code")
                    .categories("cli-agent");
                if let Some(key) = &self.api_key {
                    model = model.api_key(key.clone());
                }
                if let Some(effort) = &self.reasoning_effort {
                    model = model.reasoning_effort(effort.clone());
                }
                if let Some(max_tokens) = self.max_output_tokens {
                    model = model.max_tokens(max_tokens);
                }
                if self.prompt_cache {
                    model = model.prompt_cache(true);
                    if let Some(session_id) = &self.request_session_id {
                        model = model.session_id(session_id.clone());
                    }
                }
                Arc::new(model)
            }
            Provider::Vercel | Provider::CheaperInference | Provider::OpenAi | Provider::Local => {
                let mut model = OpenAiModel::new(self.model.as_str())
                    .base_url(self.base_url.clone())
                    .user_agent(ORCACODE_USER_AGENT);
                if let Some(key) = &self.api_key {
                    model = model.api_key(key.clone());
                }
                if let Some(effort) = &self.reasoning_effort {
                    model = if self.provider == Provider::Vercel {
                        model.nested_reasoning_effort(effort.clone())
                    } else {
                        model.reasoning_effort(effort.clone())
                    };
                }
                if let Some(max_tokens) = self.max_output_tokens {
                    model = model.max_tokens(max_tokens);
                }
                if self.provider == Provider::OpenAi && self.api_key.is_some() && self.prompt_cache
                {
                    if let Some(session_id) = &self.request_session_id {
                        model = model.prompt_cache_key(session_id.clone());
                    }
                }
                Arc::new(model)
            }
            Provider::OpenAiCodex => {
                let mut model = OpenAiCodexModel::new(
                    self.model.as_str(),
                    Arc::new(auth::CodexCliCredential::discover()),
                );
                if let Some(effort) = &self.reasoning_effort {
                    model = model.reasoning_effort(effort.clone());
                }
                if self.prompt_cache {
                    if let Some(session_id) = &self.request_session_id {
                        model = model.prompt_cache_key(session_id.clone());
                    }
                }
                Arc::new(model)
            }
        };

        // A long-running turn should survive transient provider routing,
        // connection, HTTP, and timeout failures. Keep this at the shared
        // endpoint boundary so OpenRouter, OpenAI, and local compatible
        // providers all receive the same policy.
        let retries = self.model_retries.clone();
        let settings = self.subagent_settings.clone();
        let model = RetryModel::new(model, None)
            .config(move || subagent_settings::model_retry_config(&settings))
            .gate(self.model_gates.for_endpoint(self))
            .retry_delay(orca_harness_model_providers::http_error::retry_delay)
            .on_retry(move |attempt, _max_attempts, _error| {
                retries.fetch_add(1, Ordering::Relaxed);
                // One compact notification is enough. The final run error
                // retains the provider detail if every attempt fails.
                if attempt == 2 {
                    if let Some(ui) = &ui {
                        let label = retry_label.as_deref().unwrap_or("parent");
                        let _ = ui.send(UiMsg::Notice(format!(
                            "model request failed · {label} · retrying"
                        )));
                    }
                }
            });
        Arc::new(model)
    }
}
