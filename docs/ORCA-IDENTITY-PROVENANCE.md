# FineCore — identidade e proveniência do Orca

## Identidade canônica

- Nome do projeto: **FineCore**.
- Repositório canônico: https://github.com/oigorbrito/finecore
- Upstream Orca sob avaliação: https://github.com/okikorg/orca-harness
- Escopo desta fase: identidade, atribuição, metadados e rastreabilidade; **nenhuma melhoria ou alteração funcional no código**.

## Proveniência: estado verificável

Este repositório já contém uma migração anterior, documentada em `AGENTS.md`, proveniente de `oigorbrito/core` na referência `8ca24d268ed78d71e326b1f6ac53255d1a723928`. Essa referência **não é** o commit de origem do Orca e não deve ser substituída ou reinterpretada.

A incorporação literal da árvore e do histórico de `okikorg/orca-harness` **não está demonstrada** nesta branch. O SHA de origem a ser adotado, o método de transporte Git, a correspondência de arquivos e a paridade funcional devem ser registrados em evidência separada **antes** de afirmar que FineCore é derivado do Orca.

## Procedimento de identidade

1. Preservar `main` e a branch `migration/finecore-m01`; trabalhar em branch e PR com revisão manual.
2. Inventariar a árvore atual e a árvore Orca em um SHA fixo, sem sobrescrever arquivos preexistentes.
3. Registrar os commits de origem e os métodos de incorporação; conservar atribuições e arquivos de licença/aviso aplicáveis.
4. Aplicar apenas mudanças nominais justificadas (nome exibido, documentação e metadados), evitando renomeação cega de crates, imports, APIs, variáveis, formatos persistidos e identificadores externos.
5. Verificar diff semântico, build/testes existentes e proveniência em gates agrupados; falhas ou dados ausentes permanecem explicitamente pendentes.
6. Acompanhar issues, PRs, commits de PRs e releases do upstream como **metadados**, sem aplicar patches ou fazer merge automático.

## Política de sincronização

No clone local do FineCore, usar `origin` para `oigorbrito/finecore` e `upstream-orca` para `okikorg/orca-harness`. `git fetch upstream-orca` é leitura e atualização de referências remotas; qualquer incorporação requer PR e aprovação manual. Não usar `git push --force`, `gh repo sync --force` ou atualização automática da `main`.

## Referências oficiais

- https://docs.github.com/en/pull-requests/how-tos/work-with-forks/configuring-a-remote-repository-for-a-fork
- https://docs.github.com/en/pull-requests/how-tos/work-with-forks/syncing-a-fork
- https://docs.github.com/en/repositories/creating-and-managing-repositories/renaming-a-repository

## Registro de decisão — IDENTITY-001

**Decisão:** usar `oigorbrito/finecore` como repositório canônico e `okikorg/orca-harness` como origem técnica candidata, sem afirmar incorporação já concluída.

**Motivo:** há histórico preexistente no FineCore e a substituição direta de `main` apagaria a distinção entre identidade, proveniência e código incorporado. A mudança documental é reversível, revisável por diff e não altera comportamento.

**Alternativas não adotadas nesta fase:** renomear o fork `oigorbrito/orca-harness-fork` para FineCore (nome canônico já ocupado); sobrescrever `main`; reescrever commits para trocar autoria; substituir globalmente `orca` por `finecore` nos fontes Rust.

**Evidência disponível:** README e AGENTS.md da branch de migração; metadados dos repositórios; branch e PR draft de identidade. **Evidência ausente:** SHA Orca efetivamente transportado, diff árvore-a-árvore, testes de paridade e execução comprovada do tracker na branch padrão.

**Critérios para encerrar a fase de identidade:** (a) nomes públicos coerentes; (b) proveniência antiga e nova distinguidas; (c) nenhum diff funcional; (d) atribuições preservadas; (e) revisão manual da PR. Acompanhamento de releases e execução agendada são entregas separadas, não condições presumidamente satisfeitas.

**Estado:** documentação proposta; incorporação Orca e paridade = NÃO VERIFICADAS; merge = NÃO AUTORIZADO.

## Gate consolidado — IDENTITY-02 (2026-10-23)

**Base:** `migration/finecore-m01` @ `a0b9d7755d343e284bf2c39cdbbe714ad452c8b5`. **Branch:** `docs/identity-orca-provenance`.

- Comparação GitHub: 4 commits à frente, 0 atrás; mudanças apenas em `README.md`, `AGENTS.md` e este documento (2 arquivos modificados, 1 criado). **Escopo documental: PASS**.
- Inventário observado: `packages/core-runtime` contém `package.json`, `src`, `test` e `README.md`; `scripts` contém `reconcile-finecore-m01.ps1` e `upstream_tracker.py`. Não se identificou uma árvore Rust Orca nesse inventário superficial; **ancestralidade Orca: NÃO VERIFICADA**, não “ausente” em sentido absoluto.
- `docs/migration/FINECORE-M01.md` declara migração anterior **IN PROGRESS — NOT VALIDATED** e transporte da implementação pendente. **Paridade da migração anterior: PENDENTE**.
- O workflow `.github/workflows/upstream-independent-tracker.yml` ainda usa `oigorbrito/core` como origem padrão e está na branch de migração; **acompanhamento Orca e agendamento na branch padrão: NÃO VERIFICADOS**. Nenhuma configuração operacional foi alterada nesta PR.
- **Gate de merge:** aguarda revisão humana; a ausência de alteração funcional é comprovada pelo diff de caminhos, não por benchmark de desempenho. Não foram executados benchmarks porque nenhuma hipótese de desempenho foi alterada.
