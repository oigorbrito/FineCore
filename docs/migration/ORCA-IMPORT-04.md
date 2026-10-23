# ORCA-IMPORT-04 — transporte isolado e preservação do histórico

**Status:** IMPLEMENTAÇÃO DO IMPORTADOR DISPONÍVEL; TRANSPORTE AINDA NÃO EXECUTADO.

- Origem: `okikorg/orca-harness` @ `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`.
- Base: `migration/finecore-m01`.
- Branch de trabalho: `migration/orca-tree-import`.
- Destino: `vendor/orca-harness/`, sem colisão com o conteúdo existente.
- Método: `git fetch` de SHA fixo + `git subtree add` **sem `--squash`**, preservando o commit Orca como segundo pai do commit de integração.

## Execução em clone local

Usar um clone limpo de `https://github.com/oigorbrito/finecore.git`. Selecionar a branch `migration/orca-tree-import` após buscá-la do `origin`. O script exige PowerShell 7 e `git subtree` disponível.

```powershell
git fetch origin migration/orca-tree-import
git switch migration/orca-tree-import
pwsh -File scripts/import-orca-subtree.ps1
pwsh -File scripts/import-orca-subtree.ps1 -Execute
```

A primeira execução é apenas prévia; a segunda importa localmente. **Não há push automático.** O script rejeita branch incorreta, worktree suja, origem divergente, prefixo existente e SHA Orca inesperado. Após o transporte, compara SHA da árvore importada com a árvore Git upstream, os dois pais do merge e caminhos alterados fora do prefixo. Se um gate falhar, não publicar o commit; investigar e corrigir a branch isolada.

## Gate de aceitação

1. `HEAD:vendor/orca-harness` igual a `<Orca SHA>^{tree}`.
2. Primeiro pai do commit de transporte igual ao FineCore anterior.
3. Segundo pai igual ao commit fixado do Orca.
4. Todos os arquivos alterados dentro de `vendor/orca-harness/`.
5. `main` e branch de identidade intactas; sem renomeação de crates, sem benchmarks de desempenho e sem merge automático.

A PR do importador **não representa importação executada**. O commit de transporte gerado localmente só deverá ser enviado após verificação e revisão da estratégia de incorporação.
