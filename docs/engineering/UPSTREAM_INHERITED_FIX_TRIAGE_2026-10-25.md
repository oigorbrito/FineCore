# FineCore — triagem de defeitos herdados e PRs do Orca (25/10/2026)

**Estado:** revisão somente; nenhum patch funcional transportado, nenhuma PR criada, nenhum merge.
**Base imutável desta revisão:** `integration/finecore-orca-main-review@cb143d3b343eb91c81c809ac71b221b3c20959ef`.
**Doador:** `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01` (HEAD `main` upstream na consulta).
**Migração:** `migration/orca-faithful-import@2d95a81054734730715369d54f64b28f419734fc`.

## 1. Separação de fatos

- **68 PRs upstream**: 12 abertas (`#58`, `#62` a `#72`); 54 merged e 2 fechadas sem merge (`#3`, `#42`).
- Todas as PRs já incorporadas à `main` do Orca até o commit fixado constam **do snapshot importado**. Correções já presentes incluem `#20` (tokens de cache OpenRouter), `#38` (imagens em resultados de ferramentas), `#52/#54` (falhas de fechamento de stdin), `#15/#16` (TLS/diagnóstico de transporte). Não reimportar mudanças já presentes.
- Na validação WSL/ext4, os seis testes Bun REPL falham **dos dois lados**. Não há PR aberta com patch específico para essa assinatura. As 13 ocorrências `clippy::result_large_err`, os 15 arquivos de source-size e o orçamento GNU/Linux de 6.700.000 bytes também falham em ambos. Não equivalem a defeitos exclusivos do FineCore.
- Os gates de integração focalizados passaram em Linux e macOS ([run 40250560385](https://github.com/oigorbrito/finecore/actions/runs/40250560385)); **isso não transforma gates completos herdados em PASS**.

## 2. Defeito Bun REPL: causa comum provável, ainda não corrigida

Evidência direta: `docs/validation-logs/linux-workspace-{upstream,finecore}.log`.
Os seis testes falhos e seus resultados reais:

| Teste `bun_repl::tests::` | Esperado | Obtido |
| --- | --- | --- |
| `stderr_is_returned_separately` | `out` | `> \\nout` |
| `imports_resolve_from_the_working_directory` | `42` | `> \\n42` |
| `multiline_typescript_and_top_level_await_work` | `25` | `> \\n25` |
| `reset_discards_state_without_a_second_restart_notice` | `undefined` | `> \\nundefined` |
| `crash_is_reported_and_next_call_restarts` | `fresh` | `> \\nfresh` |
| `timeout_kills_repl_and_next_call_restarts_fresh` | `undefined` | `> \\nundefined` |

O `crates/tools/src/bun_repl/output.rs::clean_stdout` do snapshot filtra redraw com ANSI CSI 2K, banners e avisos `.load`, mas não o prompt simples `> ` produzido pelo Bun 1.4.4 no ambiente observado. **Hipótese causal forte, mas sem execução do patch; não classificar como corrigido.**

**Bloco BUN-01:** instrumentar um único teste de contrato sobre stdout bruto/sentinel e reproduzir em `bun repl --no-install` no filesystem Linux; implementar normalização restrita ao prompt do protocolo (evitar apagar stdout legítimo idêntico), com regressões de conteúdo contendo `> `, ANSI, CRLF, stderr, truncamento e reinicialização. Repetir os seis casos e o workspace. Transportar como patch próprio de FineCore, separado da importação fiel. Não trocar asserts para esconder saída indevida.

## 3. PR funcional em aberto: #58 — *Workflow: retry a stage once after a transient model failure*

Fonte: https://github.com/okikorg/orca-harness/pull/58

- `OPEN`, 4 arquivos, +157/−9; modifica `crates/harness-dag/src/engine.rs`, `engine/maps.rs`, `crates/tools/src/workflow/runtime.rs` e testes de workflow.
- Problema real descrito: reset de conexão durante 100 estágios fez falhar a execução e ocultou no resultado os 94 estágios já concluídos. Patches propõem 1 retry por estágio antes do prazo e dica de `resumeFrom`.
- Os dois testes novos e experimento com endpoint fake declarados na PR dão evidência específica do caso, **não certificam ausência de duplicação de efeitos** nem representam correção das seis falhas Bun.
- Riscos para aceitação: reemitir estágio depois de streaming parcial pode duplicar ferramentas/efeitos não idempotentes; reconhecimento de erros transitórios por prefixo de mensagem; custo/latência de reexecução; deadline global, cancelamento e persistência para resume.
- **Bloco WF-58 (isolado):** comparar estado anterior/depois, injetar falha 1x/2x e falhas permanentes (auth, policy, content filter, deadline, cancellation); medir contagem de chamadas, preservar outputs terminados e demonstrar idempotência ou bloqueio explícito de retry onde houver efeitos. Apenas adotar depois desses gates. Sem auto-merge.

## 4. Dependências do plugin TypeScript gerado: lote de segurança

Arquivos-fonte: `crates/cli/src/plugin/templates/package.json` e `package-lock.json`.
Versões **efetivamente presentes no lockfile fixado**:

| PR aberta | Pacote | Lock atual | Candidato | Evidência/relevância | Tratamento |
| --- | --- | --- | --- | --- | --- |
| [#69](https://github.com/okikorg/orca-harness/pull/69) | `proxy-addr` | 2.0.7 | 2.0.8 | [GHSA-jqcg-44mw-7w3h](https://github.com/advisories/GHSA-jqcg-44mw-7w3h): spoof de IP sob determinadas configurações de confiança de proxy, severity Critical | Prioridade alta, validar exposição |
| [#66](https://github.com/okikorg/orca-harness/pull/66) | `source-map-js` | 1.2.1 | 1.2.2 | [GHSA-68fv-2mgg-jv7q](https://github.com/advisories/GHSA-68fv-2mgg-jv7q): DoS por source-map indexado hostil, High | Prioridade alta; confirmar entrada explorável |
| [#68](https://github.com/okikorg/orca-harness/pull/68) | `fast-uri` | 3.1.5 | 3.1.9 | [GHSA-w74x-v6f9-j3xg](https://github.com/advisories/GHSA-w74x-v6f9-j3xg): esquema URI percent-encoded, Moderate | Prioridade alta; testar normalização |
| [#67](https://github.com/okikorg/orca-harness/pull/67) | `@modelcontextprotocol/sdk` | 1.30.0 | 1.31.0 | OAuth issuer-binding e mudanças nos tokens armazenados | Validar interoperabilidade e migração |
| [#71](https://github.com/okikorg/orca-harness/pull/71) | `hono` | 4.13.2 | 4.13.13 | Atualização transitiva de servidor | Validar rotas e teste do plugin |
| [#70](https://github.com/okikorg/orca-harness/pull/70) | `ip-address` | 10.5.0 | 10.7.4 | Correções de validação/parse | Validar casos de IPv4/IPv6 |
| [#72](https://github.com/okikorg/orca-harness/pull/72) | `vitest` | 2.1.9 | 5.0.5 | Upgrade **major** do framework de testes e substituição de transientes | Tratar em bloco separado |

**ATENÇÃO:** #66–#72 modificam o **mesmo** `package-lock.json`. A aplicação literal sucessiva dos lockfiles das PRs individuais pode reverter atualizações anteriores. Consolidar em **uma única instalação/resolução reproduzível** e verificar hashes, `npm ci` (sobre plugin realmente gerado), build, testes, smoke MCP e `npm audit` em fixture isolada. Não executar geração/instalação não confiável no diretório canônico nem publicar sem revisão. Criticidade da CVE **não prova exploração do FineCore**: a superfície está num template JS usado por plugins gerados, não no binário Rust diretamente.

## 5. GitHub Actions em aberto — trabalho de manutenção, não correção do core

| PR | Escopo | Decisão |
| --- | --- | --- |
| [#62](https://github.com/okikorg/orca-harness/pull/62) | `softprops/action-gh-release` 2.6.2→3.0.3; release do Orca | Revisão apenas: workflow de release upstream permanece bloqueado em FineCore |
| [#63](https://github.com/okikorg/orca-harness/pull/63) | `actions/upload-artifact` 4.6.2→7.0.2 | Avaliar compatibilidade de runner/artefatos em CI isolada |
| [#64](https://github.com/okikorg/orca-harness/pull/64) | `actions/checkout` 4.4.0→7.0.1 | Avaliar runner/Node e revisão de pin SHA |
| [#65](https://github.com/okikorg/orca-harness/pull/65) | `actions/download-artifact` 4.3.0→8.0.2; release do Orca | Revisão apenas, sem habilitar publicação |

Manter workflows que gastam API ou publicam release bloqueados para o repositório upstream original. Não transformar atualizações de actions em permissão para publicar/mergear.

## 6. Ordem de trabalho e gates

**A. BUN-01** (defeito funcional herdado, seis falhas com assinatura comum). Resultado esperado: seis testes PASS com Bun nativo, sem falsos positivos de limpeza de stdout e sem alterações de identidade.

**B. TEMPLATE-SEC-01** (as seis PRs #66–#71 como **um único lote**). Resultado esperado: lockfile consolidado e instalação/build/teste do plugin gerado em ambiente isolado; sem vulnerabilidades conhecidas nos pacotes-alvo e sem quebra MCP.

**C. WF-58** (retry de estágio, PR aberta). Resultado esperado: experimentos determinísticos com falha transitória/permanente, isolamento de efeitos, custo de retry, contract tests do `resumeFrom`; não assumir melhoria apenas pelo título da PR.

**D. TEMPLATE-TEST-72** (Vitest major). Resultado esperado: suite JS validada no novo major; trabalho isolado do lote de segurança.

**E. DEBT-GATES** (13 clippy, 15 source-size e limite de binário). Separar por custo/benefício; mudanças em `HarnessError`, layout de crate ou otimização de tamanho exigem testes de compatibilidade e medição; não inflar a migração fiel com refactor incidental.

Os trabalhos devem ser conduzidos em branches separadas, originadas da candidata de integração validada. Cada patch demanda diff mínimo, testes focalizados, verificações de regressão de workspace, CI transparente e revisão humana. O upstream tracker de issues/PRs/commits e a `main` ficam intocados; merge somente manual após autorização explícita.

**Não demonstrado:** ganho de latência, custo por tarefa correta ou qualidade FineCore vs Pi. Nada neste relatório substitui benchmark pareado.
