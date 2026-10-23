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
