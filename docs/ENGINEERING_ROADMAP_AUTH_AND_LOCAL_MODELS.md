# Roadmap de engenharia: autenticação, providers e modelos locais

Estado: **proposta documental, com uma mudança de Fase 4 implementada fora da sequência** (login do Codex por navegador; ver seção K). As demais fases não foram iniciadas.
Data de referência: 2026-11-18 (atualização de 2026-11-16). Commit de referência do EvCore: `636b34f` (2026-11-18). `a2b35b8` é ancestral desse commit e era a referência original.

Este documento é permanente. Leia-o antes de alterar autenticação, providers, credenciais ou modelos locais, e antes de propor qualquer mudança nessas áreas. Ele não autoriza implementação: cada fase tem um gate de aprovação.

---

## A. Objetivo e princípios

Objetivo: auditar, medir e, se justificado, evoluir a autenticação, os providers e a integração com modelos locais do EvCore, sem perder funcionalidade existente.

Princípios obrigatórios:

1. **Decisões baseadas em evidência reproduzível.** Toda afirmação sobre comportamento precisa de um comando, um arquivo, um commit e um resultado registrados.
2. **Confiabilidade operacional antes de novidade.** Falha previsível e recuperável vale mais que recurso novo.
3. **Simplicidade de uso e de manutenção.** Mudança que aumenta a superfície de configuração precisa justificar o aumento.
4. **Preservação integral das funcionalidades existentes.** Nenhum método de login, provider, adapter ou flag é removido sem equivalência comprovada e aprovação explícita.
5. **Compatibilidade retroativa sempre que possível.** Credenciais, configurações e sessões já gravadas por usuários devem continuar funcionando.
6. **Mudanças incrementais, isoladas e reversíveis.** Uma PR, um escopo, um plano de reversão.
7. **Nenhuma implementação baseada apenas em preferência arquitetural.** Preferência não é evidência.
8. **Nenhuma duplicação de funcionalidade existente.** Antes de criar um módulo, prove que não há um equivalente no repositório.

---

## B. Contexto arquitetural

Fatos que todo agente deve ter em mente:

- **O EvCore é uma cópia quase integral do Orca Harness (upstream).** A origem, os arquivos alterados e a lista de exceções estão em `PROVENANCE.md`. A cópia foi importada no commit `0de23d3` a partir do upstream `2a9fd399…` (árvore `9c35d0dd…`). Esses valores vêm do `PROVENANCE.md` e **não foram rechecados nesta tarefa**; a Fase 0 deve recheckar.
- **Funcionalidades herdadas não contam como originais do EvCore.** Ao relatar algo como “implementado no EvCore”, verifique se já existia no upstream.
- **O Orca upstream é a baseline de referência** para identificar o que é herdado, divergente ou exclusivo.
- **Pi e “Marshall” são referências externas de comparação técnica**, não bases para substituição integral. Atenção: **“Marshall” não aparece em nenhum arquivo do repositório**; o nome e o repositório de referência precisam ser confirmados antes da Fase 0. Não assuma o que é esse projeto.
- **Toda comparação deve registrar o SHA exato de cada repositório examinado** (EvCore, Orca, Pi, Marshall), a data da coleta e o comando usado.
- **Não presuma que EvCore e Orca permanecem idênticos.** Isso deve ser demonstrado pela comparação da Fase 0, não afirmado.

---

## C. Mapa atual (baseline observada)

Este mapa descreve o que foi observado no código em 2026-11-16. Não é conclusão de auditoria.

### C.1 Autenticação

| Item | Onde | Observação |
| --- | --- | --- |
| Despacho de autenticação | `crates/cli/src/auth/mod.rs` | Só `Provider::OpenAiCodex` tem OAuth. Os demais retornam erro “does not use OAuth”. |
| Device flow genérico | `crates/cli/src/auth/oauth_device.rs` (95 linhas) | Inicia com `device_auth_id`/`user_code`, faz polling, retorna `authorization_code` e `code_verifier`. Só HTTP 403 é tratado como “pendente” (`is_pending`). |
| Login OpenAI Codex | `crates/cli/src/auth/openai_codex/mod.rs` (546 linhas em 2026-11-18); fluxo de navegador em `browser.rs`, `callback.rs` e `pkce.rs` | Usa o device flow. `max_polls` = 120; timeout de requisição = 15 s. O intervalo de polling é o do servidor + 3 s (padrão 5 + 3 = 8 s), o que dá cerca de 16 min no pior caso. |
| Descoberta de credencial | `openai_codex/mod.rs`, função `discover` | Usa `~/.config/orcacode/auth/openai-codex.json` (ou `$XDG_CONFIG_HOME`) se o arquivo existir; senão, `$CODEX_HOME/auth.json` ou `~/.codex/auth.json`. Home: `HOME` e, se ausente, `USERPROFILE` (desde 2026-11-18). Sem home, o caminho do Codex fica vazio, nunca relativo. |
| Renovação de token | `openai_codex/mod.rs`: `current`, `recover_unauthorized`, `exchange`, `persist_tokens` | Renova 300 s antes do vencimento. Rotaciona o `refresh_token` e grava de volta no arquivo. |
| Coordenação de renovação | `openai_codex/mod.rs:23-29`, `tokio::sync::Mutex` | Coordena apenas dentro do mesmo processo. Não há lock entre processos (`flock`, `fd-lock`, `fs2` não aparecem em `crates/`). |
| Persistência privada | `crates/cli/src/auth/store.rs` | Diretório 0700, arquivo 0600 em Unix, escrita em temporário com `create_new` e `rename`. Exige caminho absoluto. |
| Contrato de credencial | `crates/provider-auth/src/lib.rs` | `BearerCredential`, `CredentialSource` (`credential`, `refresh`) e `CredentialErrorKind` (Missing, Malformed, Expired, Refresh, Storage). |

**Adicionado em 2026-11-18** (seção K): fluxo de navegador com Authorization Code e PKCE S256 (`pkce.rs`), servidor de callback em loopback com validação de `state` (`callback.rs`, `TcpListener` em `127.0.0.1`), montagem da URL e troca de código (`browser.rs`). Não há fallback de colagem manual.

**Ainda não encontrado no código** (verificado por busca em `crates/`):

- Integração com keychain ou cofre do sistema (`keyring`, `keychain`, `secret-service`).

### C.2 Credenciais de API e configuração

| Item | Onde | Observação |
| --- | --- | --- |
| Arquivo de configuração | `crates/cli/src/config/storage.rs` | `config.json` em `ORCA_CONFIG_DIR`, `$XDG_CONFIG_HOME/orcacode`, `~/.config/orcacode` ou `%APPDATA%\orcacode` (Windows). |
| Chaves de API | mesmo arquivo, seção `api_keys` | Gravadas em texto puro no JSON, com permissão de dono. Não há cofre do sistema. |
| Formato documentado | comentário no topo de `storage.rs` | Qualquer mudança de formato precisa atualizar esse comentário e manter leitura retroativa. |

### C.3 Providers

Fonte: `crates/cli/src/msg.rs` (enum `Provider`, linhas 12–20; URLs na linha 55 em diante).

| Provider (rótulo) | Adapter | Observação |
| --- | --- | --- |
| `openrouter` | `crates/model-providers/src/openrouter/` | OpenAI-compatible. |
| `vercel` | `crates/model-providers/src/vercel/` | Gateway. |
| `cheaperinference` | `crates/model-providers/src/cheaperinference/` | Gateway. |
| `openai` | `crates/model-providers/src/openai/` | OpenAI-compatible. |
| `openai-codex` | `crates/model-providers/src/openai_codex/` | Endpoint Codex com credencial OAuth. |
| `anthropic` | `crates/model-providers/src/anthropic/` | Protocolo nativo. |
| `local` | adapter OpenAI-compatible (`openai/`) | Padrão `http://localhost:11434/v1` (Ollama). Modelo padrão `qwen3.5:9b`. |

### C.4 Modelos locais

- **Ollama:** URL padrão em `crates/cli/src/msg.rs:67` e `crates/cli/src/run_args.rs:246`. A janela de contexto é lida pela API nativa `/api/show` em `crates/cli/src/context_window.rs` (função `ollama_context_window`, linha 57; movida de `main.rs` desde a baseline).
- **llama.cpp:** não há adapter específico. A compatibilidade vem do adapter OpenAI-compatible (comentário em `crates/model-providers/src/openai/mod.rs:4`).
- **Comportamento de fallback a verificar:** em `crates/cli/src/run_args.rs:245-246`, o provider `openai` sem chave cai em `http://localhost:11434/v1`; com chave, usa `https://api.openai.com/v1`. Pode ser intencional. Não altere sem evidência e sem registrar a decisão.

### C.5 Testes existentes (contagem por `#[test]` e `#[tokio::test]`)

- Autenticação (`crates/cli/src/auth/`, `crates/provider-auth/`): 19 funções em 2026-11-18 (eram 7 antes da mudança de login; as novas cobrem PKCE, callback, browser e caminho de home). Inclui `unauthorized_recovery_reuses_a_concurrently_rotated_token` e `atomic_persistence_preserves_unknown_fields`.
- Adapters (`crates/model-providers/`): 138 funções.
- Configuração (`crates/cli/src/config/`): 22 funções.
- Ignorados que tocam o tema: benchmarks `request_bench.rs` (OpenAI e Codex); `tui/tests/skills.rs:264` (rede: clona repositório e chama modelo).

**Lacunas aparentes (a confirmar na Fase 2):**

- Nenhum teste de login real, nem de fluxo ponta a ponta do device flow contra servidor simulado.
- Nenhum teste de concorrência entre processos.
- Nenhum teste de caminho de credencial quando `HOME` não existe (Windows).
- Porta ocupada: coberta no nível do `bind` (`busy_port_is_reported_as_address_in_use`). O fallback completo para o device flow **não** é testado de ponta a ponta.
- Cancelamento libera a porta: coberto (`cancelled_wait_releases_the_port`).
- Conexão ociosa atrasa o callback válido em 5 s: observado (seção K.3), não corrigido.

---

## D. Hipóteses de risco (a testar, não conclusões)

Cada item abaixo é uma hipótese levantada pela leitura do código. Nenhuma foi reproduzida ainda.

- **H1 — Renovação concorrente entre processos.** Duas instâncias do `orcacode`, ou o Codex CLI oficial, podem renovar o mesmo `refresh_token` ao mesmo tempo. Como a rotação invalida o token antigo, uma das sessões pode ficar sem credencial. Teste: Fase 2, “refresh concorrente”.
- **H2 — Escrita no arquivo compartilhado do Codex CLI.** Quando a credencial vem de `~/.codex/auth.json`, a renovação grava de volta nesse arquivo (`persist_tokens`). Isso pode afetar a sessão do Codex CLI oficial. Teste: Fase 2, “reinicialização durante persistência” e “credenciais corrompidas”.
- **H3 — `HOME` ausente no Windows.** *Atualização (2026-11-18): parcialmente mitigado. `home_dir()` usa `HOME`, depois `USERPROFILE`, e aceita só caminhos absolutos. `codex_auth_path()` não produz mais `auth.json` relativo. Evidência: teste `home_tests` (2 passam) e gravação real do arquivo em 2026-11-18.* `codex_auth_path` e `orcacode_auth_path` dependem de `HOME`. No Windows, `HOME` costuma não estar definido. Nesse caso, `codex_auth_path` retorna `auth.json` relativo, e `discover` pode ler um `auth.json` do diretório atual (o workspace). Isso seria um problema de segurança e de compatibilidade. Teste: Fase 2 e Fase 1 (compatibilidade Windows e Linux).
- **H4 — Pendência do device flow.** O código trata somente HTTP 403 como “pendente”. A RFC 8628 descreve pendência por código de erro (`authorization_pending`). O comportamento real do provedor precisa ser confirmado na documentação oficial, não por suposição. Teste: Fase 1 e Fase 2.
- **H5 — Ausência de cofre de credenciais.** Chaves de API ficam em texto puro em `config.json`, protegidas só por permissão de arquivo. É a baseline e não é defeito por si só. Mudança depende da Fase 4, item 1, e de plano de migração.

---

## E. Fases do roadmap

Cada fase tem entrega e gate. Um gate só é marcado como concluído com evidência anexada.

### Fase 0 — Baseline

**Objetivo:** tornar o estado atual reproduzível.

Passos:

1. Registrar SHA e branch de cada repositório: EvCore, Orca upstream, Pi e Marshall (após confirmação do nome e da URL).
2. Comparar EvCore com o upstream. Classificar cada arquivo como **herdado** (idêntico ao upstream), **divergente** (alterado) ou **exclusivo** (criado no EvCore). Usar `git diff --stat` e `git diff --name-status` entre os SHAs registrados.
3. Inventariar autenticação, providers, credenciais e modelos locais (seção C deste documento, atualizada com os SHAs).
4. Identificar testes existentes e lacunas (seção C.5, revisada por `cargo test -- --list` no ambiente registrado).
5. Registrar ambiente: sistema operacional, versão do Rust (`rust-toolchain.toml`), comandos executados e saídas.

**Entrega:** relatório de baseline com SHAs, comandos e resultados. Gate: relatório revisado por humano.

### Fase 1 — Auditoria de autenticação

**Objetivo:** comparar a autenticação do EvCore (herdada do Orca), do Pi e do Marshall contra referências normativas.

Critérios de comparação:

- OAuth 2.0 e o fluxo usado (device, Authorization Code, PKCE).
- Validação de `state`.
- Callback local e loopback.
- Segurança de credenciais em disco e em memória.
- Renovação de tokens e rotação de `refresh_token`.
- Concorrência entre tarefas e entre processos.
- Cancelamento, timeout e recuperação.
- Compatibilidade Windows e Linux.

Referências normativas:

- **RFC 9700** — OAuth 2.0 Security Best Current Practice.
- **RFC 8252** — OAuth 2.0 for Native Apps (inclui loopback).
- **RFC 7636** — PKCE.
- **RFC 8628** — Device Authorization Grant.
- **RFC 6749** — OAuth 2.0 (base).
- Documentação oficial de cada provedor, usada apenas para o comportamento do endpoint.

**Entrega:** matriz técnica comparativa, com cada célula apontando para arquivo e linha. Gate: matriz revisada.

### Fase 2 — Testes empíricos

**Objetivo:** executar cenários equivalentes em todas as implementações.

Cenários obrigatórios:

1. Login normal.
2. Primeiro login e login subsequente.
3. Token expirado.
4. Falha de rede.
5. Callback inválido (quando aplicável).
6. Porta ocupada (quando aplicável).
7. Timeout.
8. Cancelamento.
9. Refresh concorrente (H1).
10. Reinicialização durante persistência (H2).
11. Credenciais corrompidas.
12. Caminho de credencial sem `HOME` (H3).

Classificação obrigatória de cada teste:

- **Unitário:** sem rede, sem arquivo real do usuário.
- **Integração simulada:** servidor de teste local que imita o provedor.
- **Autenticação real:** conta real, feita manualmente e registrada.

Para cada execução, registrar: ambiente, versões, comando exato, resultado, limitações.

**Regra de registro:** teste que não pôde rodar é registrado como **não verificado**, nunca como aprovado.

**Entrega:** relatório reproduzível de testes. Gate: relatório revisado.

### Fase 3 — Decisão técnica

**Objetivo:** classificar cada funcionalidade e aprovar a especificação antes de qualquer código.

Classificações:

- **KEEP:** preservar como está.
- **IMPROVE:** aperfeiçoar pontualmente.
- **ADOPT:** adotar solução externa com vantagem demonstrada.
- **REJECT:** rejeitar mudança sem benefício comprovado.

Cada decisão precisa de:

- Estado atual.
- Evidência (arquivo, linha, teste, resultado).
- Problema identificado.
- Alternativa avaliada.
- Benefício esperado, medido ou verificável.
- Riscos.
- Impacto em funcionalidades existentes.
- Testes de aceitação.
- Plano de reversão.

**Entrega:** especificação técnica aprovada. Gate: **aprovação explícita do responsável antes da Fase 4.**

### Fase 4 — Evolução de autenticação

Só implementar o que foi aprovado na Fase 3. Ordem de prioridade:

1. **Persistência e proteção de credenciais.** Inclui avaliar H2 e H5.
2. **Coordenação de renovação de tokens.** Inclui H1, com lock entre processos.
3. **Browser OAuth,** somente se tecnicamente justificado pela Fase 1.
4. **Fallbacks e recuperação.** Inclui H3.
5. **Compatibilidade com os fluxos existentes.**

Restrições:

- Não remover device flow nem qualquer método funcional sem comprovar equivalência e obter aprovação explícita.
- Não migrar credenciais de forma destrutiva. Migração precisa ser aditiva, reversível e testada com arquivos reais de versões anteriores.

**Entrega:** PRs pequenas, isoladas e testadas, uma por item.

### Fase 5 — Modelos locais

Auditoria independente e conservadora. Não misturar com mudanças de autenticação.

Preservar: providers existentes, adapters OpenAI-compatible, integração Ollama, integração llama.cpp, configurações e fluxos funcionais.

Avaliar, sem alterar, possíveis melhorias de descoberta de modelos, metadados e compatibilidade. Verificar especialmente o fallback de `run_args.rs:245-246` (C.4).

Não fazer, sem evidência concreta e aprovação:

- Substituir adapters.
- Alterar protocolos.
- Adicionar gerenciamento de download, load ou unload de modelos.
- Alterar defaults (URL `localhost:11434`, modelo `qwen3.5:9b`).

**Entrega:** relatório de compatibilidade e, se necessário, proposta de melhorias para aprovação.

### Fase 6 — Validação final

Verificar:

- Preservação de funcionalidades.
- Compatibilidade das credenciais existentes (arquivos de `config.json` e `auth/openai-codex.json` gerados por versões anteriores).
- Funcionamento de cada provider.
- Funcionamento dos modelos locais.
- Testes de regressão (`make check` e `make test`, conforme `AGENTS.md`).
- Compatibilidade entre Windows e Linux.
- Comparação com a baseline da Fase 0.
- Documentação atualizada.

**Entrega:** relatório de aceitação. Itens não verificados aparecem como **não verificados**.

---

## F. Critérios de aprovação

Não aprovar mudança que:

- Remova funcionalidade sem autorização.
- Apresente regressão.
- Exija migração destrutiva de credenciais.
- Duplique módulo existente.
- Aumente complexidade sem benefício demonstrado.
- Dependa exclusivamente de benchmark sintético.
- Não tenha plano de reversão.

Metas de desempenho não podem ser fixadas antes da coleta da baseline da Fase 0.

Preservação de funcionalidades é obrigatória. Teste não executado vira **não verificado**, nunca **aprovado**.

---

## G. Regras de execução para agentes

1. Ler este documento antes de trabalhar em autenticação, providers, credenciais ou modelos locais.
2. Não iniciar implementação durante auditorias.
3. Não alterar `main` durante a avaliação.
4. Não fazer merge automaticamente.
5. Não misturar escopos independentes (por exemplo, autenticação e modelos locais).
6. Não modificar configurações ou formatos de credenciais sem plano de compatibilidade aprovado.
7. Não marcar gates como concluídos sem evidência anexada.
8. Solicitar aprovação antes de avançar da especificação (Fase 3) para a implementação (Fase 4).
9. Preservar alterações locais existentes e nunca sobrescrevê-las.
10. Não gravar chaves, tokens ou senhas em arquivos, testes ou documentação. Use arquivos de teste temporários e apague-os.
11. Não rodar benchmarks ao vivo (que usam `OPENROUTER_API_KEY` ou outras chaves) sem pedido explícito.

---

## H. Evidências exigidas

Toda afirmação sobre comportamento deve trazer:

- **Fonte:** arquivo e linha, ou URL oficial com data de acesso.
- **Commit:** SHA do repositório examinado.
- **Comando:** exato, incluindo variáveis de ambiente relevantes (sem valores secretos).
- **Resultado:** saída ou resumo, com data.
- **Limitação:** o que não foi possível verificar.

Números de desempenho também precisam de máquina, versão e dado bruto ou manifesto versionado (regra já presente em `AGENTS.md`).

---

## I. Referências

Internas:

- `AGENTS.md` e `CLAUDE.md` (instruções do agente).
- `CONTRIBUTING.md` (revisão humana de `crates/harness-core`).
- `PROVENANCE.md` (origem, lista de exceções, nomes de compatibilidade).
- `crates/cli/src/auth/`, `crates/provider-auth/`, `crates/cli/src/config/storage.rs`, `crates/model-providers/`, `crates/cli/src/msg.rs`, `crates/cli/src/run_args.rs`.

Normativas:

- RFC 6749 — The OAuth 2.0 Authorization Framework.
- RFC 7636 — Proof Key for Code Exchange (PKCE).
- RFC 8252 — OAuth 2.0 for Native Apps.
- RFC 8628 — OAuth 2.0 Device Authorization Grant.
- RFC 9700 — Best Current Practice for OAuth 2.0 Security.

Externas (SHA de HEAD coletado em 2026-11-16 via `git ls-remote`; a Fase 0 deve fixar o SHA exato examinado):

- Orca Harness (upstream): `https://github.com/okikorg/orca-harness`. Baseline do EvCore: `2a9fd399…`. HEAD coletado: `d89bf274c225075ae3f3b1f305a4717e05990ab7`.
- Pi: `https://github.com/earendil-works/pi`. HEAD coletado em 2026-11-16: `c8c456c85c6e0256be620ab73dc59d3522dbfeaa`. HEAD em 2026-11-18 (`git ls-remote`): `f6d0c2f9b6ff...` (SHA completo a fixar na Fase 0).
- Marshall (`agention-marshall`): `https://github.com/LaurentZuijdwijk/agention-marshall`. HEAD coletado: `d4a5410ece38fdccf52dea0c5655148cba040fdf`.

---

## J. Pendências abertas


1. Recheckar, na Fase 0, o SHA e a árvore do upstream declarados em `PROVENANCE.md`. (Não verificado em 2026-11-18.)
2. Confirmar o comportamento do provedor no device flow (H4) pela documentação oficial.
3. Decidir se o fallback de `openai` sem chave (C.4) é intencional.
4. Teste de login real com conta ChatGPT, repetido e registrado. Feito uma vez em 2026-11-18; a falha de gravação já está corrigida. A troca de código com erro forçado no navegador não foi testada.
5. Confirmar que o servidor aceita `originator=evcore` (R6 da spec). Evidência parcial: a troca de código funcionou em 2026-11-18.
6. Teste de compatibilidade de credenciais entre versões (seção 7 da spec): não feito.
7. Fallback manual (colar URL ou código): não implementado.
8. Gate de Fase 3 para o restante da Fase 4: não aprovado (seção K.1).

---

## K. Registro de gates e mudanças

### K.1 Gates

| Gate | Estado em 2026-11-18 | Evidência |
|---|---|---|
| Fase 0 (baseline) | **Não concluída.** Parcial: SHAs registrados no cabeçalho e na seção I. Recheck do upstream pendente. | Seção J.1 |
| Fase 1 (auditoria) | **Não concluída.** | Nenhuma matriz em `docs/plan/` |
| Fase 2 (testes empíricos) | **Não concluída.** Parcial: testes unitários do login e uma execução manual real (K.3). | Seção K.3 |
| Fase 3 (decisão técnica) para **login por navegador** | **Aprovada pelo responsável em 2026-11-16**, decisões 1 a 4 (cliente, porta, dependências, método padrão). | `docs/plan/2026-11-16-openai-codex-browser-login.md`, seção 5. Registro: mensagens do responsável no chat de 2026-11-16 ("confirmo", "copiar do pi", "go"). |
| Fase 3 para o **restante da Fase 4** (persistência, lock entre processos, fallbacks) | **Não aprovada.** | Nenhuma spec aprovada |
| Fase 4 (login por navegador) | **Implementada, não mesclada.** Fora da sequência: Fases 0 a 2 não concluídas. | Seção K.2 |

Observação: o gate da Fase 3 para o login por navegador foi dado depois de uma especificação escrita pelo agente. As Fases 1 e 2 que o roadmap exige antes da Fase 4 não foram executadas. Esse desvio fica registrado aqui e não é corrigido por retroação.

### K.2 Mudança de 2026-11-18: login do Codex por navegador

- Arquivos novos: `crates/cli/src/auth/openai_codex/pkce.rs`, `callback.rs`, `browser.rs`.
- Arquivos alterados: `openai_codex/mod.rs`, `crates/cli/Cargo.toml` (`ring = "0.17"`, feature `net` do tokio), `Cargo.lock` (apenas a aresta `orcacode -> ring`), `run_args.rs` (texto de ajuda).
- Correções no mesmo ciclo:
  - Caminho de home (H3): `USERPROFILE` como fallback e caminho vazio quando não há home.
  - Página de sucesso só após a troca de token (HTTP 502 se a troca falhar).
- Login real: uma execução em 2026-11-18. O arquivo `~/.config/orcacode/auth/openai-codex.json` existe, com data de 2026-11-18 00:12. Só existência e data foram verificadas; o conteúdo não foi lido.

### K.3 Verificação e reprodução

Comandos executados em Windows com Rust 1.99.1, em 2026-11-18. O resultado observado vem depois de `#`.

```
cargo test -p orcacode --bin orcacode auth::        # 19 passam
cargo test -p orcacode --bin orcacode home_tests    # 2 passam
cargo fmt --all --check                             # exit 0
bash ci/check-source-size.sh                        # exit 0, sem saída
python3 scripts/audit_crates.py --top 25            # "No files over 599 lines."
cargo clippy -p orcacode --all-targets              # exit 0, nenhum aviso em crates/cli/src/auth/
cargo clippy --workspace --all-targets -- -D warnings  # exit 101: 13 erros, todos em crates/harness-core (não alterado)
cargo test -p orcacode                              # 626 passam, 10 falham (ver abaixo)
```

Falhas da suíte `cargo test -p orcacode`:
- 9 falhas (plugins, imagens, política de shell, instruções) também ocorrem no commit base `636b34f`, verificado em worktree separado em 2026-11-18.
- `tui::tests::slash_theme_opens_a_picker_preselected_on_the_active_theme`: falhou só na suíte completa. Passou isoladamente duas vezes e no subconjunto `tui::tests` (somente as 3 falhas de imagem). Tratada como **intermitente** (provável estado global de tema). **Não testada no commit base**, então não há evidência de que seja anterior a esta mudança.

Observações reais (não automatizadas):
- Redirect `localhost` no Windows: o callback chegou ao listener `127.0.0.1` e o código foi trocado.
- Conexão ociosa na porta 1455: o callback válido levou 5,0 s (medido com teste temporário, removido depois).
- Arquivo de credencial criado em 2026-11-18 00:12 após o login pelo navegador.
- Porta ocupada: o fallback para o device flow está descrito na mensagem de código, mas não foi observado em execução real.

Limitações: `make` não está instalado neste Windows. Os comandos acima reproduzem `make check` e `make test` manualmente. `make test` também executa `benchmarks/run_reporting_tests.sh`, que não foi executado. Teste de compatibilidade de credenciais e fallback manual não foram feitos.
