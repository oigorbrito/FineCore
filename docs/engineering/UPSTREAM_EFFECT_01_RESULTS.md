# FineCore — UPSTREAM-EFFECT-01 (resultado pareado)

**Data:** 2026-10-26  
**Status:** medição concluída; **não promover a correção #110 sem mitigação de efeitos não idempotentes**.  
**Execução canônica:** [GitHub Actions 40397574006](https://github.com/oigorbrito/finecore/actions/runs/40397574006), **SUCCESS**.  
**Diagnóstico independente:** [40397275996](https://github.com/oigorbrito/finecore/actions/runs/40397275996), **SUCCESS**.

## Pergunta experimental

A mudança ainda aberta no [Orca #58](https://github.com/okikorg/orca-harness/pull/58), transportada de forma fiel para [FineCore #110](https://github.com/oigorbrito/finecore/pull/110), melhora a recuperação de workflows sem produzir regressões de segurança operacional?

## Comparação reproduzida

| Braço | SHA exato | Compilação | SHA-256 do teste Rust |
|---|---|---|---|
| FineCore base | `2f980015a733ec449ca1ea8be51e97da08220fb1` | `target-baseline` | `32b7b619c2f6053c9708e4b149b09f7e5b6720c23f88afb89d13509155f07c48` |
| FineCore com retry | `092ff1ff0fb654c814663a296318abc06266584f` | `target-candidate` | `4ab39c8c90a131325830e9c09adb1a3e2d70e34fdae0517ecd238679359d9a57` |

Mesma máquina Ubuntu 24.04, Rust 1.99.1, `--locked`, mesmo arquivo de instrumentação copiado nos dois checkouts; recompile independente de `orca-harness-dag` e `orca-harness-tools` confirmada pelo log. A instrumentação executa uma ferramenta Rust `FnTool` que anexa uma linha a um **arquivo real**. O mock de modelo retorna erros controlados antes ou depois da ferramenta. São **20 repetições por cenário por braço**, 8 cenários, **160 workflows por braço**. Analisador rejeita amostras ausentes/duplicadas e contadores inconsistentes. Ambos os testes Rust e a reconciliação passaram.

## Resultados observados

| Cenário (20 trials cada) | Conclusão na base | Conclusão com retry | Efeitos da ferramenta na base | Efeitos com retry |
|---|---:|---:|---:|---:|
| Saudável, com ferramenta | 20/20 | 20/20 | 20 | 20 |
| Transporte transitório após executar ferramenta (uma falha) | 0/20 | **20/20** | 20 | **40** |
| Transporte após ferramenta (duas falhas) | 0/20 | 0/20 | 20 | **40** |
| Resposta incompleta após ferramenta (uma falha) | 0/20 | **20/20** | 20 | **40** |
| Autenticação após ferramenta | 0/20 | 0/20 | 20 | 20 |
| Transporte antes de qualquer ferramenta (uma falha) | 0/20 | **20/20** | 0 | 0 |
| Resposta incompleta antes da ferramenta (uma falha) | 0/20 | **20/20** | 0 | 0 |
| Autenticação antes da ferramenta | 0/20 | 0/20 | 0 | 0 |
| **Total, distribuição artificial uniforme** | **20/160** | **100/160** | **100** | **160** |

**Diferença medida nesse conjunto controlado:** +80 conclusões; +60 efeitos externos de ferramenta; 60 dos 160 workflows no candidato executaram uma ferramenta **duas vezes** (zero na base), incluindo os 20 em que a tarefa ainda falhou após a segunda tentativa. Invocações de `Model::generate`: **260** na base e **420** no candidato (+160). Essas contagens **não** são equivalentes a tokens, dólares, tempo de resposta ou qualidade de programação.

O retry melhora a **recuperação diante de falhas transitórias simuladas**, mas reemite a etapa inteira mesmo que uma operação não idempotente já tenha ocorrido. A repetição foi comprovada por escrita em arquivo, não apenas por contador interno. A autenticação permaneceu terminal e os casos saudáveis ficaram inalterados.

## Decisão técnica

- **Ganho: demonstrado**, condicionado à incidência de erros transitórios e aos tipos de estágio.
- **Risco: demonstrado**, duplicação de ferramenta/efeito externo após falha parcial; em cenários como cobranças, envios ou operações não idempotentes, é bloqueador operacional.
- **Parecer:** manter FineCore PR #110 Draft; **não aprovar o mecanismo tal como está** para execução geral. Repetição pode exigir opt-in restrito a operações comprovadamente idempotentes, ou mecanismo persistente de deduplicação por operação, com testes de recuperação depois de falha e reinicialização.
- **Portas de aprovação futuras:** preservar recuperação segura antes da execução de ferramentas; nenhuma duplicação após efeitos externos; política explícita para efeitos de modelos/provedores e custos; casos de autenticação, cancelamento e deadline sem retry; sem regressão nas suítes herdadas.
- **Não medido:** taxa real de falhas em produção, tarefas de programação com modelo real, qualidade da solução, latência sob rede, custo por tarefa correta, provedor real e sistemas externos distribuídos. O conjunto é deliberadamente artificial, não uma estimativa de incidência na população de usuários. Sem adoção de um modelo causal/populacional, **não extrapolar os 20/160 e 100/160 para a produção**.

## Integridade da investigação e falha corrigida

Execuções anteriores da mesma instrumentação (`40396351571`, `40396459159` e `40396947104`) usaram um **único `CARGO_TARGET_DIR` compartilhado**: o segundo braço executou o binário de testes compilado no primeiro. Suas comparações de eficácia são **INVÁLIDAS**, independentemente dos seus status de testes. A investigação detectou isso comparando as linhas `Compiling` e os executáveis efetivamente invocados. O workflow foi corrigido para diretórios separados e verificou os SHA-256 distintos acima.

O diagnóstico `40397275996`, com **somente o candidato compilado isoladamente**, já demonstrava recuperações e duplicações; o A/B independente `40397574006` confirmou essas observações em ambos os braços com o mesmo fixture. Logs, contadores por amostra, hashes e relatório de análise estão preservados nos artefatos dos Actions. O relatório exclui alegações de ganho baseadas nos runs contaminados.

## Outros trabalhos atualmente abertos no Orca

- [Orca #66–#72](https://github.com/okikorg/orca-harness/pulls): atualizações das dependências Node. FineCore [Draft #112](https://github.com/oigorbrito/finecore/pull/112) já apresenta medida concreta para segurança dos templates (17 findings no snapshot original → 0 no lock corrigido; matriz de 5 sistemas/versões PASS). São ganhos de segurança e compatibilidade, **não** evidência de melhoria do motor de programação.
- Orca #62–#65: mudanças de versões de GitHub Actions (`release-orcacode`, `ci`, `bench`, `bench-live`). Não alteram o runtime Rust; várias linhas de release permanecem inativas no FineCore por guarda de repositório. Requerem validação de CI e compatibilidade próprias; nenhuma vantagem em produtividade de agentes foi medida.
- Orca `main` ainda no snapshot `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`: mudanças já integradas nesse snapshot **não** são incrementais em relação à migração FineCore.

**Governança:** Nenhum merge, publicação, alteração de política, mutação do upstream ou chamada paga a modelo/provedor ocorreu nesta investigação. Experimento mantido em Draft PR #119.
