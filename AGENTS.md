# AGENTS.md

Instruções para agentes de programação neste repositório (EvCore). Os fatos abaixo foram conferidos no código e nos docs em 2026-11-18. Quando um fato mudar, atualize este arquivo na mesma mudança.

## Contexto

- EvCore é um agente de código para terminal, derivado do Orca Harness. A origem e as diferenças estão em `PROVENANCE.md`.
- Workspace Rust. A versão em `Cargo.toml` (`[workspace.package]`) é 0.9.0.
- Dois binários do mesmo crate `orcacode`: `evcore` (entrada pública) e `orcacode` (compatibilidade).
- Não há release. Não adicione tags, workflows de release, publicação ou passos que enviem binários. Ver `CONTRIBUTING.md` e `ci/README.md`.

## Filosofia de engenharia

- Quero decisões de engenharia baseadas em evidências reproduzíveis, priorizando confiabilidade operacional, simplicidade de uso e manutenção, sem sacrificar funcionalidades.
- Não confunda código que compila, testes automatizados aprovados e produto operacionalmente validado. São níveis diferentes de evidência.

## Nome canônico

- O nome do produto é **EvCore**. Use esse nome em texto novo e em qualquer texto visível que você alterar.
- Não renomeie nomes de compatibilidade listados em `PROVENANCE.md` (seção "Not changed"): crates `orca-harness-*` e `orcacode`, variáveis `ORCA_*`, caminhos `orcacode/` de configuração, sessão e credenciais, o user agent `orcacode/<versão>` e identificadores de plugin. Renomear quebra configurações e credenciais existentes.
- Não altere `LICENSE`, `NOTICE`, `PROVENANCE.md`, referências ao upstream nem os registros em `benchmarks/results/`.

## Comandos verificados

Fonte: `Makefile` e `.github/workflows/ci.yml`.

| Comando | O que faz |
| --- | --- |
| `make check` | fmt, gate de tamanho de arquivo e clippy (o que o CI roda antes dos testes) |
| `make test` | `cargo test --workspace` e os testes de relatório de benchmark |
| `make build` | `cargo build --release -p orcacode` |
| `make help` | lista todos os alvos |

Os alvos `notes`, `prepare`, `tag`, `release`, `dist` e `publish` estão desabilitados de propósito. Não os use nem os reabilite sem decisão registrada em `PROVENANCE.md`.

## Regras de edição

- **`crates/harness-core` é o kernel.** Não edite sem pedido explícito, e sempre com revisão humana depois. Se for preciso propor uma mudança, escreva um patch em `patch/harness-core/` para revisão, sem aplicá-lo direto. Ver `CONTRIBUTING.md` e `patch/harness-core/README.md`.
- **Nomes de compatibilidade não mudam:** crates `orcacode` e `orca-harness-*`, variáveis `ORCA_*`, caminhos de configuração, sessão e credenciais, e o user agent `orcacode/<versão>`. Lista completa em `PROVENANCE.md`, seção "Not changed".
- **Instaladores e updater do upstream não são usados.** `orcapods.ai` e os scripts de instalação não servem ao EvCore. `evcore update` deve falhar, por design.
- **Arquivos de referência do upstream não são editados.** `docs/releases/` é histórico do Orcacode. `docs/external/` tem regras próprias de sincronização descritas no README da pasta.
- **Planos vão em `docs/plan/`.** Ao concluir ou mudar o estado de um plano, atualize o cabeçalho com estado e data.

## Autenticação, providers e modelos locais

- Antes de alterar `crates/cli/src/auth/`, `crates/provider-auth/`, `crates/cli/src/config/`, `crates/model-providers/`, `crates/cli/src/msg.rs`, `crates/cli/src/run_args.rs` ou credenciais, leia `docs/ENGINEERING_ROADMAP_AUTH_AND_LOCAL_MODELS.md`.
- Esse roadmap é documental. Não inicie implementação nessas áreas sem aprovação da especificação (Fase 3).

## Limite de tamanho de arquivo

- Cada `.rs` em `crates/` deve ter menos de 600 linhas (`ci/check-source-size.sh`, limite 599).
- **Estado atual (2026-11-18): o gate passa.** `ci/check-source-size.sh` sai com código 0 e `scripts/audit_crates.py` não lista nenhum arquivo acima de 599 linhas. O maior é `crates/cli/src/skills.rs` (597 linhas).
- Arquivos acima de 550 linhas estão perto do limite. Ao crescer um deles, divida a mudança em módulos antes de passar de 599.

## Evidência em docs e código

- Número de desempenho só entra em documentação com data, commit, versão e máquina, e com o dado bruto ou o `manifest.json` do run no repositório.
- Não apresente números medidos no upstream como medidas do EvCore. Diga a origem em cada tabela.
- Antes de dar um doc por concluído, verifique os links relativos. Se mexer no manual, rode `python3 docs/external/check.py`.
- `benchmarks/harness-comparison/fixture/` contém dados de benchmark. Os `AGENTS.md` e `CLAUDE.md` dali valem só para o fixture, não para este projeto.

## Operação

- Não faça commit nem push sem pedido. A branch de trabalho é `claude/tender-hawking-dcgx72`.
- Não rode os benchmarks ao vivo (`benchmarks/harness-comparison/run.py` sem `--dry-run`) sem pedido. Eles usam `OPENROUTER_API_KEY` e geram custo.
- Não grave chaves, tokens nem senhas em arquivos, testes, exemplos ou documentação.
- Antes de afirmar que algo passa no CI, rode o comando correspondente localmente, ou diga que não rodou.
