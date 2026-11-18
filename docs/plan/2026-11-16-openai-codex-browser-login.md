# Login OpenAI Codex por navegador (PKCE + loopback), método portado do Pi

Estado: **implementação inicial concluída, não mesclada.** Decisões 1 a 4 aprovadas pelo responsável em 2026-11-16. Pendências: fallback manual (seção 11), teste com conta real e revisão humana.
Data: 2026-11-16. Commit de referência do EvCore: `636b34f` (branch `claude/tender-hawking-dcgx72`). O roadmap `docs/ENGINEERING_ROADMAP_AUTH_AND_LOCAL_MODELS.md` cita `a2b35b8`; a Fase 0 deve reconciliar.

Referência de método: Pi, `https://github.com/earendil-works/pi`, commit `05fd3fbda0bbb9a1c7a76e4bf92ab58ed77c8d94` (licença MIT). Arquivos lidos: `packages/ai/src/auth/oauth/openai-codex.ts`, `callback-server.ts`, `pkce.ts`. O método foi reimplementado em Rust; nenhum trecho TypeScript entrou no repositório.

---

## 1. Objetivo

Adicionar ao `openai-codex` um login pelo navegador (Authorization Code com PKCE S256 e redirecionamento para loopback), mantendo o device flow como fallback, sem alterar o formato de `auth/openai-codex.json` nem o caminho de descoberta de credenciais.

## 2. Não objetivos

- Não remover nem alterar a lógica do device flow (`crates/cli/src/auth/oauth_device.rs`, `openai_codex/mod.rs`).
- Não adotar lock entre processos, cofre de sistema ou outras melhorias do roadmap (Fase 4, itens 1 e 2). São specs separadas.
- Não tocar em outros providers nem em Pi/Marshall como base de código.
- Não tocar em `crates/harness-core`.

## 3. Fluxo

1. Gerar `code_verifier` com 32 bytes aleatórios, em base64url sem padding. Gerar `challenge = base64url(SHA-256(verifier))`, com `code_challenge_method=S256`.
2. Gerar `state` com 16 bytes aleatórios, em hexadecimal.
3. Abrir listener em `127.0.0.1:1455`, caminho `/auth/callback`. A porta é fixa porque o redirect registrado precisa bater.
4. Montar a URL de autorização e abrir o navegador (melhor esforço). A URL também é exibida ao usuário.
5. No callback:
   - Rota diferente de `/auth/callback` ou método diferente de GET: 404, continua aguardando.
   - `state` diferente: 400, continua aguardando.
   - Parâmetro `error`: falha o login.
   - Sem `code`: 400, continua aguardando.
   - `code` válido: troca por token em `https://auth.openai.com/oauth/token` com `code_verifier` e o mesmo `redirect_uri` da autorização. Persiste pelo caminho existente.
6. Fallback manual (colar URL ou código): **não implementado nesta etapa** (ver seção 11).
7. Timeout de 5 min. Cancelamento pela tarefa de login encerra o listener.
8. Renovação de tokens: inalterada.

## 4. Arquivos afetados

Novos, em `crates/cli/src/auth/openai_codex/` (dentro do módulo Codex, para não criar um módulo genérico sem uso fora dele):

- `pkce.rs`: verifier, challenge e `state`.
- `callback.rs`: listener loopback com validação de `state`, timeout e cancelamento.
- `browser.rs`: montagem da URL, abertura do navegador, espera e troca do código.

Alterados:

- `openai_codex/mod.rs`: `login` tenta o navegador e cai para o device flow se a porta estiver ocupada. Troca de código generalizada (`exchange_authorization_code`), usada pelos dois fluxos.
- `Cargo.toml` do `cli`: `ring = "0.17"` e feature `net` do tokio.
- `Cargo.lock`: apenas a nova aresta `orcacode -> ring` (sem pacote novo).
- `run_args.rs`: texto de ajuda sobre o método de login.

Não alterados: `crates/provider-auth/`, `crates/cli/src/auth/store.rs`, `crates/cli/src/config/storage.rs`, `crates/harness-core/`.

## 5. Decisões

Decisões 1 a 4 **aprovadas** pelo responsável ("confirmo", "copiar do pi"). Interpretação registrada: "copiar do pi" significa seguir o comportamento do Pi (cliente, porta, método padrão e dependências mínimas), sem trazer código TypeScript.

- **1 — Cliente:** mesmo `client_id` e redirect do Pi. É o mesmo `CLIENT_ID` que o device flow do EvCore já usa. O risco de identidade permanece (R6).
- **2 — Porta:** 1455, como no Pi.
- **3 — Dependências:** `ring = "0.17"` para SHA-256 e aleatoriedade. A crate já está no `Cargo.lock` via rustls, na mesma versão, então nenhum pacote novo é baixado. Feature `net` do tokio.
- **4 — Método padrão:** navegador, como no Pi. O device flow entra como fallback quando a porta 1455 está ocupada.

Decisão 5 (device flow e `state`): fora de escopo, registrada para não misturar com H4 do roadmap.

Decisão 6 (licença): o método foi reimplementado e não copiado. Se algum trecho for adaptado no futuro, incluir o aviso MIT do Pi em `NOTICE` e registrar a origem em `PROVENANCE.md`.

## 6. Riscos

- **R1 — Conflito com o Codex CLI.** O mesmo `client_id` e a mesma conta usados em paralelo podem invalidar um `refresh_token` (H1 e H2 do roadmap). Não resolvido aqui.
- **R2 — Porta ocupada.** Coberto pelo fallback para device flow.
- **R3 — Navegador em outra máquina.** Sem fallback manual, o login só conclui se o navegador alcançar `localhost:1455` na máquina do EvCore. Limitação conhecida.
- **R4 — Protocolo não documentado.** Endpoints e parâmetros vêm do Pi, não de documentação oficial. Verificar antes de mesclar (pendência J.2 do roadmap).
- **R5 — Segurança do callback.** Escuta só em `127.0.0.1`. `state` obrigatório. Páginas de resposta estáticas, sem eco de dados da requisição. Nenhum log de `code`, `code_verifier` ou token.
- **R6 — Identificador `originator`.** O Pi envia `originator=pi`. Esta implementação envia `originator=evcore`, porque enviar `pi` faria o EvCore se apresentar como Pi. **Não verificado** se o servidor aceita esse valor. Se recusar, a correção é escolher o valor aceito por decisão explícita, não voltar a usar `pi` por padrão.

## 7. Testes

**Unitários (sem rede, sem arquivo do usuário):**
- PKCE: challenge conferido com o vetor da RFC 7636, apêndice B. Formato do verifier (43 caracteres) e do `state` (32 hex).
- URL de autorização: contém `code_challenge`, `code_challenge_method=S256`, `state`, `redirect_uri` e `client_id`.
- Callback: código válido; `state` errado e rota errada ignorados até um callback válido; erro do provedor falha; timeout.

**Integração simulada:** não implementada ainda. Servidor que imite `auth.openai.com` para a troca de código.

**Autenticação real:** **não executada.** Conta real, feita manualmente e registrada.

**Compatibilidade:** credencial gravada pelo novo fluxo deve ser lida pelo fluxo atual, e vice-versa, com um `auth/openai-codex.json` de versão anterior copiado para diretório temporário. **Não executada.**

Teste não executado é registrado como **não verificado**.

## 8. Critérios de aceitação

- Testes do módulo `openai_codex` passam.
- `clippy` sem avisos nos arquivos novos.
- `rustfmt --check` limpo nos arquivos tocados.
- `ci/check-source-size.sh` sem arquivos acima do limite.
- Formato de `openai-codex.json` e caminhos de descoberta inalterados.
- Nenhum arquivo em `crates/harness-core/` alterado.

## 9. Plano de reversão

- Trabalho em PR isolada, sem merge automático.
- Reverter a PR restaura o device flow como único método. Não há migração de dados, porque o formato de credencial não muda.

## 10. Sequência de entrega

1. Aprovação das decisões 1 a 4. **Feito.**
2. Fase 0 mínima: fixar SHAs e localizar o ponto de entrada. **Feito parcialmente** (SHAs na cabeça deste documento).
3. PKCE e `state`, com testes. **Feito.**
4. Listener loopback, com testes. **Feito** (testes unitários; sem teste de integração simulada).
5. Troca de código e persistência, com teste de compatibilidade. **Troca feita; compatibilidade não testada.**
6. Fallback manual. **Não feito.**
7. Ponto de entrada na CLI e documentação. **Feito** (ajuda em `run_args.rs`).
8. Teste manual com conta real, registrado. **Não feito.**

## 11. Estado da implementação

Arquivos em `crates/cli/src/auth/openai_codex/`:

- `pkce.rs`: verifier de 32 bytes, challenge S256 e `state` hexadecimal. Teste com o vetor da RFC 7636.
- `callback.rs`: listener em `127.0.0.1`, valida método, caminho e `state`. Responde páginas estáticas. Timeout de 5 min. Quatro testes.
- `browser.rs`: monta a URL de autorização, abre o navegador (melhor esforço), espera o callback e troca o código. Um teste da URL.
- `mod.rs`: `login` tenta o navegador. Se a porta 1455 estiver ocupada, mostra aviso e usa o device flow. A troca de código é compartilhada pelos dois fluxos.

Verificação local (2026-11-16, Windows):

- `cargo test -p orcacode --bin orcacode openai_codex`: 12 testes passam.
- `cargo clippy -p orcacode --all-targets`: sem avisos nos arquivos de auth.
- `rustfmt --check` limpo nos arquivos tocados.
- `ci/check-source-size.sh`: saída vazia e código 0.
- `cargo test -p orcacode`: 622 passam e 9 falham. As mesmas 9 falham no commit base `636b34f`, verificado em worktree separado. Estão em plugins, imagens, política de shell e instruções. Não foram investigadas.

Desvios em relação às seções 3 e 5:

- **Fallback manual não implementado.** O login roda em tarefa de fundo, sem canal de entrada na TUI. Colar URL ou código exige entrada interativa, fora desta mudança.
- **Seleção de método na TUI não implementada.** O Pi pergunta o método. Aqui o padrão é o navegador, e o device flow só entra por fallback.

Pendências antes de mesclar:

- Teste de login real com conta ChatGPT, registrado. **Não executado.**
- Confirmar que o servidor aceita `originator=evcore` (R6). **Não verificado.**
- Confirmar o comportamento de `localhost` no Windows: o redirect usa `localhost` e o listener usa `127.0.0.1`, como no Pi. **Não verificado.**
- Teste de compatibilidade de credenciais (seção 7). **Não executado.**
- Revisão humana da PR.

## 12. Teste manual (2026-11-17)

- Navegador exibiu "Signed in to OpenAI. You may close this page." **Isso só confirma que o callback recebeu o código.** A página é enviada antes da troca por token.
- Após o teste, `~/.config/orcacode/auth/openai-codex.json` **não existia**. Logo, a troca de código não gravou credencial. **Causa identificada:** a TUI mostrou `login failed: no absolute configuration home is available for OAuth credentials`, depois de `authorization approved · exchanging credentials…`. Logo, a troca de código **funcionou** (o servidor aceitou `originator=evcore` e o `redirect_uri`). A falha veio na gravação: `orcacode_auth_path()` só lia `HOME`, que não existe no Windows (risco H3 do roadmap).
- **Correção:** `home_dir()` usa `HOME` e, se ausente, `USERPROFILE`. Só caminhos absolutos. `codex_auth_path()` não cai mais em `auth.json` relativo ao diretório atual quando não há home.
- Testes: `home_tests` (2) e `openai_codex` (14) passam.
- Pendente: repetir o login e confirmar a gravação em `%USERPROFILE%\.config\orcacodeuth\openai-codex.json`.
- `~/.codex/auth.json` não foi alterado (última modificação em 13/11/2026), como esperado.
- **Gravação confirmada (2026-11-18, 00:12):** `~/.config/orcacode/auth/openai-codex.json` existe após o login pelo navegador. Só existência e data foram verificadas; o conteúdo não foi lido, de propósito.
- Pendente: teste de chat usando essa credencial com `CODEX_HOME` apontando para pasta vazia, e teste de compatibilidade (seção 7).

## 13. Correção: página de sucesso só após a troca de token (2026-11-18)

- A página "Signed in" agora é enviada somente depois que `exchange_authorization_code` retorna sucesso. Se a troca falhar, o navegador recebe HTTP 502 com a página "OpenAI sign-in failed. Return to EvCore for the error details."
- `callback.rs`: `wait_for_callback` recebe a etapa `complete` e escreve a resposta conforme o resultado.
- Testes: `failed_complete_shows_failure_not_success` (502, sem texto de sucesso), `returns_code_and_shows_success_after_complete` (200). Total do módulo: 15 passando.
- **Não verificado:** repetir o login real com a troca forçada a falhar e confirmar a página de erro. Não foi feito.
