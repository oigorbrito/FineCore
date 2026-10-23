# ORCA-06 — Inventário de identidade e contratos (baseline)

Data: 2026-10-23  
Base imutável: `41be0191bdefce512943204e13fa93a5df12703c`  
Doador: `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`  
Árvore doadora e importada: `9c35d0dd0d7e0f226063f16dd80161e231628e2b`

## Escopo e método

Inventário remoto dos dez manifests `crates/*/Cargo.toml`, `Cargo.toml` raiz, `README.md`, `Makefile` e workflow `.github/workflows/ci.yml`, todos lidos da referência imutável. A árvore Git recursiva do doador contém 1.284 entradas, `truncated=false`. Esta etapa não altera arquivos em `vendor/orca-harness/`, não executa build, não afirma compatibilidade funcional e não executa benchmarks.

## Nomes de pacotes e superfícies

| Caminho | Cargo package | Tipo / risco |
| --- | --- | --- |
| crates/cli | orcacode | binário `orcacode`; CLI, scripts e instalação: público |
| crates/harness-core | orca-harness-core | núcleo e dependências Rust: público |
| crates/harness-dag | orca-harness-dag | DAG e dependências Rust: público |
| crates/provider-auth | orca-harness-provider-auth | contrato de autenticação: público |
| crates/model-providers | orca-harness-model-providers | provedores e dependências Rust: público |
| crates/sandbox-providers | orca-harness-sandbox-providers | sandbox e dependências Rust: público |
| crates/tools | orca-harness-tools | ferramentas e dependências Rust: público |
| crates/tool-extensions | orca-harness-tool-extensions | integrações e dependências Rust: público |
| crates/extensions | orca-harness-extensions | extensões e dependências Rust: público |
| crates/sdk | orca-harness-sdk | SDK e dependências Rust: público |

As bibliotecas com `[lib] name` explícito usam identificadores `orca_harness_*`. Não converter automaticamente nomes com hífen em identificadores de importação Rust sem validar cada manifest.

## Outros contratos observados

- Workspace: versão `0.9.0`, edição Rust `2021`, licença `Apache-2.0`, `repository=https://github.com/okikorg/orca-harness`, `homepage=https://orcapods.ai/orcacode`.
- CLI e documentação: `orcacode`, flags `--plan`, `-p`, `--json`, `--bare`, `--no-session`, `--anthropic`, `--openrouter`, modos `normal/plan/orchestrate/auto/yolo`; todos sujeitos a testes antes de mudanças.
- SDK documentado: `orca_harness_sdk::{Harness, OpenAiModel, ToolPreset}`. É uma superfície de compilação de consumidores.
- Provedores documentados: OpenAI/Codex, Anthropic, OpenRouter e Ollama local; nomes de variáveis como `OPENAI_API_KEY`, `ANTHROPIC_API_KEY` e `OPENROUTER_API_KEY` são interfaces de configuração.
- Distribuição upstream: scripts de instalação, `orcacode update`, tags `orcacode-v*`, `Makefile` com targets `release`, `publish`, `tag`; não reutilizar para distribuir FineCore sem análise.
- CI do doador roda a partir da raiz upstream. Arquivos de workflow dentro de `vendor/` não equivalem automaticamente a workflows executáveis no repositório FineCore.

## Classificação de mudança

**A — Congelado:** todo conteúdo `vendor/orca-harness/`, nomes de crates, imports Rust, flags, variáveis de ambiente, formato de sessão, permissões, caminhos persistidos, atualizador e protocolo headless. Não renomear nesta fase.

**B — Permitido fora do doador:** documentação de arquitetura, atribuição e proveniência, inventários de contratos e eventual camada de integração FineCore isolada e opt-in.

**C — Exige RFC e testes de compatibilidade:** qualquer alias de binário, pacote publicável, alteração de manifest, nome de API, armazenamento, protocolo, autenticação, configuração ou release.

## Gates de aceitação

1. Integridade: `HEAD:vendor/orca-harness` deve continuar igual a `9c35d0dd0d7e0f226063f16dd80161e231628e2b`.
2. Isolamento: alterações ORCA-06 apenas fora de `vendor/orca-harness/`.
3. Identidade: FineCore é a camada canônica de composição; o nome Orca identifica a origem e seus contratos preservados.
4. Sem afirmação de paridade: build, testes e comparação de desempenho pertencem ao ORCA-07.
5. Sem merge automático; revisão manual antes de integrar.

## Pendências para ORCA-06B

Inventariar no código (não apenas documentação) nomes de comandos, diretórios de configuração, variáveis de ambiente, formatos persistidos, eventos NDJSON e protocolos de extensão; produzir matriz símbolo → localização → consumidor → compatibilidade, com evidência reproduzível. Só depois decidir se alguma adaptação é necessária.


## ORCA-06B — Matriz de contratos observados no código

Inspeção estática remota na referência `41be0191`. Localizações abaixo são relativas a `vendor/orca-harness/`; números de linha referem-se ao commit fixado. Consumidores externos são *potenciais*, não foram enumerados por telemetria ou busca global.

| Símbolo / contrato | Fonte (linha) | Consumidor / persistência | Classe |
| --- | --- | --- | --- |
| `ORCA_CONFIG_DIR` | `crates/cli/src/config/storage.rs:41-54` | CLI e configuração persistida | A |
| `XDG_CONFIG_HOME`, `APPDATA`, `HOME` | `crates/cli/src/config/storage.rs:45-52` | resolução de diretório multiplataforma | A |
| `config.json` | `crates/cli/src/config/storage.rs:54` | configurações e aprovações | A |
| `memory.sqlite3` | `crates/cli/src/config/storage.rs:64-65` | banco de memória | A |
| `stored_approvals`, `save_approval`, `remove_approval` | `crates/cli/src/config/storage.rs:246-287` | autorização por workspace | A |
| `stored_mcp_servers`, `save_mcp_server` | `crates/cli/src/config/storage.rs:302-375` | configurações de servidores MCP | A |
| `ORCA_MODEL`, `ORCA_BASE_URL` | `crates/cli/src/run_args.rs:11-12` | execução e seleção de modelo | A |
| `ORCA_SUBAGENT_DEPTH` | `crates/cli/src/run_args.rs:28` | limite de profundidade de subagentes | A |
| `ORCA_THEME` | `crates/cli/src/run_args.rs:40` | aparência da CLI | A |
| `orcacode resume ID` | `crates/cli/src/run_args.rs:48` | CLI e scripts externos | A |
| `--json` e serialização de eventos | `crates/cli/src/headless.rs:1-3,112-128` | consumidores de stdout NDJSON | A |
| metadados de sessão serializados | `crates/cli/src/runtime/session.rs:80` | persistência de sessões | A |
| `write_private_json` | `crates/cli/src/auth/store.rs:10-36` | armazenamento privado de autenticação | A |

**A = contrato congelado nesta fase.** A leitura estática confirma pontos de entrada e persistência, mas não valida o esquema completo de eventos, a migração de sessões ou a compatibilidade entre versões.

### Recomendações vinculantes para revisão

1. Não substituir `ORCA_*` por `FINECORE_*` por busca e troca.
2. Não mudar o nome do executável, imports, crates, caminhos de configuração, arquivos de sessão ou formato NDJSON.
3. Manter documentação FineCore fora da árvore doadora; qualquer wrapper futuro deve ser opt-in, com testes de não regressão e sem publicação upstream acidental.
4. ORCA-07 deverá separar `cargo fmt --check`, `cargo clippy`, `cargo test --workspace`, testes de parsers de benchmark e `cargo bench -- --test` (smoke), de benchmarks de eficácia/desempenho propriamente ditos.
5. Antes de declarar cobertura completa de ORCA-06B, realizar busca sistemática no restante da árvore Rust para todas as variáveis `ORCA_*`, diretórios, flags, eventos e esquemas de persistência; o presente inventário é uma amostra dirigida a módulos críticos.


## ORCA-06C — Gate de integridade do inventário

O scanner em `scripts/audit-orca-contracts.py` exige três condições cumulativas: árvore `HEAD:vendor/orca-harness` igual ao SHA fixado, `git status --porcelain` limpo sob o prefixo importado, e lista dos `*.rs` idêntica à listagem rastreada por `git ls-tree`. A comparação de árvore isolada **não** demonstra que os bytes do checkout estejam intactos.

A árvore remota fixada contém **517 arquivos Rust** (incluindo 2 arquivos de benchmark fora dos crates), segundo o inventário Git recursivo de 1.284 entradas, não truncado. Essa é uma expectativa de cobertura, não um resultado de execução do scanner.

**Execução ainda não comprovada:** os testes Python e o inventário completo não foram executados em runner. A criação de workflow GitHub Actions foi bloqueada pelo mecanismo de permissões disponível; não se deve relatar CI verde, saída JSON ou cobertura real antes de uma execução comprovada. O checkout completo do repositório privado não está acessível no ambiente de execução desta sessão.

### Gate executável

```bash
python3 scripts/test_audit_orca_contracts.py
python3 scripts/audit-orca-contracts.py --output orca-contracts.json
python3 -c 'import json; x=json.load(open("orca-contracts.json")); assert x["files_scanned"] == 517 and x["matches"] > 0; print(x["files_scanned"], x["matches"])'
```

Após obter o JSON real, classificar cada ocorrência por API pública, configuração, persistência, protocolo, identidade apenas textual ou teste. Não confundir frequência de palavras com consumidores efetivos nem usar o inventário estático como benchmark.


## ORCA-06D — Contratos com consumidores comprovados (leitura do código fixado)

Os itens seguintes foram confirmados por leitura direta do código em `41be0191`, não por contagem de palavras. Caminhos relativos a `vendor/orca-harness/`.

| Interface e evidência | Consumidor/efeito comprovado | Risco de mudança |
| --- | --- | --- |
| `crates/cli/src/config/storage.rs:41-65`: `ORCA_CONFIG_DIR` > `XDG_CONFIG_HOME` > `APPDATA` (Windows) / `HOME/.config`; `config.json`, `sessions`, `memory.sqlite3` | `config_path()`, `sessions_dir()`, `memory_path()`; dados locais persistidos | **Crítico**: isolamento, recuperação e perda aparente de dados |
| `crates/cli/src/config/storage.rs:242-280`: `approvals[workspace]` | `stored_approvals`, `save_approval`, `remove_approval`; falha de leitura resulta em lista vazia (fail-closed) | **Crítico**: autorização e segurança |
| `crates/cli/src/config/storage.rs:296-325`: entradas MCP como string legada ou objeto `{command,enabled}` | `stored_mcp_servers()` aceita ambos os formatos | **Alto**: retrocompatibilidade de configuração |
| `crates/cli/src/run_args.rs:10-40,92-115`: variáveis `ORCA_MODEL`, `ORCA_BASE_URL`, `ORCA_SUBAGENT_DEPTH`, `ORCA_THEME` e flags de execução | `parse_run_args()` transforma parâmetros em `Config` | **Alto**: scripts e automações |
| `crates/cli/src/main.rs:370-376`: `ORCA_MODEL_CONCURRENCY` | `Endpoint::from_config()` passa limite a `subagent_settings::configured()` | **Alto**: paralelismo e recursos |
| `crates/cli/src/runtime/session.rs:11-33,35-59`: retomada por ID/prefixo e workspace | `SessionHandler::resume` e `SessionHandler::create`; ID ambíguo falha explicitamente | **Crítico**: integridade e retomada de sessões |
| `crates/sdk/src/session/persistence.rs:14-23,36-44`: extensão `recovery.json` | `TruncationStore::load/save` do SDK | **Alto**: recuperação do contexto |
| `crates/cli/src/headless.rs:112-131`: `--json` | `EventStream` serializa `HarnessEvent` como JSON por linha no stdout | **Crítico**: consumidores externos do protocolo |
| `crates/cli/src/headless.rs:133-163`: saída sem `--json` | texto do assistente em stdout, raciocínio/ações em stderr | **Alto**: parsers e redirecionamentos |
| `crates/cli/src/main.rs:134-160`: `--continue`, `--resume`, `--no-session`, `--json`, `.orca/`, `AGENTS.md` | interface pública documentada no help compilado | **Alto**: CLI e instruções por workspace |

### Decisões e limites

- Preservar a precedência de diretórios e os formatos de configuração legados, inclusive o formato MCP anterior.
- Não modificar a semântica fail-closed de aprovações, nem os critérios de ambiguidade de sessão.
- Não interceptar stdout do modo `--json` com banners ou mensagens de identidade FineCore; isso quebraria consumidores NDJSON.
- A serialização de `HarnessEvent` está comprovada, mas a **estrutura completa de cada variante** e sua estabilidade entre versões ainda exigem leitura de `crates/extensions` e testes de contrato.
- Este gate comprova **consumidores internos específicos**, não a enumeração exaustiva dos consumidores externos.
- A execução integral de `scripts/audit-orca-contracts.py` permanece pendente; não há artefato de runner que autorize declará-la aprovada.


## ORCA-06E — Esquemas de eventos e sessões: evidência de fonte

Leitura do commit fixado `41be0191`:

- `crates/extensions/src/events.rs:18-71`: `HarnessEvent` é `Serialize/Deserialize` com `#[serde(tag = "type", rename_all = "snake_case")]`. Variantes: `agent_start`, `assistant_delta`, `reasoning_delta`, `tool_input_delta`, `assistant`, `tool_call`, `tool_started`, `tool_finished`, `tool_result`, `usage`, `result`, `error`. Campos variam por variante; por exemplo, `tool_result` carrega `tool_call_id`, `tool_name`, `output` e `is_error`. A serialização para stdout é confirmada em `crates/cli/src/headless.rs:123-131`.
- `crates/extensions/src/events.rs:44-56`: `tool_started` pode ocorrer mais de uma vez em caso de retry e `tool_finished` pode chegar fora da ordem de chamadas. Consumidores não podem assumir pareamento 1:1 ordenado.
- `crates/extensions/src/session.rs:1-8,21-38`: sessões são JSONL append-only; primeira linha é `SessionMeta` com `v=1`, `id`, `created_at`, `workspace`, `model` e `parent` opcional.
- `crates/extensions/src/session.rs:140-183`: versões incompatíveis são rejeitadas; somente a última linha truncada e sem terminador pode ser descartada com aviso. Erros em outras linhas são tratados como corrupção.
- `crates/sdk/src/session/persistence.rs:36-44`: armazenamento de recuperação em caminho com extensão `recovery.json`.

**Invariantes de compatibilidade para integração futura:** preservar discriminador `type`, nomes e tipos dos campos, eventos repetidos/fora de ordem, cabeçalho de sessão v1, semântica de corrupção e recuperação de última linha. Não traduzir nomes de variantes para a identidade FineCore no fio.

**Limite:** a matriz de esquemas está sustentada pela leitura de fonte; não representa teste de serialização executado nem compatibilidade empírica com clientes externos.
