#Requires -Version 5.1
[CmdletBinding()]
param(
  [string]$Workspace = (Join-Path $env:USERPROFILE 'finecore-migration'),
  [switch]$Push
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$source = 'https://github.com/oigorbrito/core.git'
$destination = 'https://github.com/oigorbrito/finecore.git'
$sourceBranch = 'feat/core-runtime-foundation-001'
$targetBranch = 'migration/finecore-m01'
$sourcePin = '8ca24d268ed78d71e326b1f6ac53255d1a723928'
$destinationMainPin = 'd932e68ecdf49cc1d51b344aa629659cd46d0997'
function Invoke-Git {
  & git @args
  if ($LASTEXITCODE -ne 0) { throw "Git failed ($LASTEXITCODE): git $($args -join ' ')" }
}
function Git-Value {
  $result = & git @args
  if ($LASTEXITCODE -ne 0) { throw "Git failed ($LASTEXITCODE): git $($args -join ' ')" }
  return ($result | Out-String).Trim()
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw 'Git is required.' }
if (Test-Path -LiteralPath $Workspace) { throw "Refusing to reuse an existing workspace: $Workspace" }
$parent = Split-Path -Parent $Workspace
New-Item -ItemType Directory -Force -Path $parent | Out-Null
Invoke-Git clone --branch $targetBranch --single-branch --no-local $destination $Workspace
Push-Location $Workspace
try {
  if ((Git-Value status --porcelain).Length -gt 0) { throw 'Dirty checkout; refusing to merge.' }
  $targetHead = Git-Value rev-parse HEAD
  $mainRemote = Git-Value ls-remote $destination refs/heads/main
  if (($mainRemote -split '\s+')[0] -ne $destinationMainPin) { throw 'Destination main moved; re-audit before proceeding.' }
  Invoke-Git fetch --no-tags $source "refs/heads/$sourceBranch"
  $fetched = Git-Value rev-parse FETCH_HEAD
  if ($fetched -ne $sourcePin) { throw "Source pin changed: $fetched" }
  Invoke-Git merge-base --is-ancestor $sourcePin FETCH_HEAD
  # Explicitly preserve both parent histories; do not use squash, rebase, or force.
  & git merge --allow-unrelated-histories --no-ff --no-commit $sourcePin
  if ($LASTEXITCODE -ne 0) {
    Write-Warning 'Merge did not complete cleanly. Conflicts remain in the workspace; nothing was pushed.'
    throw 'Resolve conflicts manually, or run git merge --abort. Do not force the merge.'
  }
  $conflicts = @(git diff --name-only --diff-filter=U)
  if ($conflicts.Count -gt 0) { throw "Unmerged paths: $($conflicts -join ', ')" }
  $sourcePaths = @(git ls-tree -r --name-only $sourcePin)
  $missing = @($sourcePaths | Where-Object { -not (git ls-files --error-unmatch -- $_ 2>$null) })
  if ($missing.Count -gt 0) { throw "Source files missing: $($missing -join ', ')" }
  $merged = Git-Value rev-parse -q --verify MERGE_HEAD
  if ($merged -ne $sourcePin) { throw 'Unexpected merge parent.' }
  Invoke-Git commit -m 'feat(migration): reconcile pinned Core history into FineCore'
  $commit = Git-Value rev-parse HEAD
  $parents = Git-Value rev-list --parents -n 1 HEAD
  if (($parents -split '\s+').Count -ne 3) { throw 'Expected a two-parent merge commit.' }
  Write-Host "MERGE_COMMIT=$commit"
  Write-Host "SOURCE_PIN=$sourcePin"
  Write-Host "PREVIOUS_FINECORE_HEAD=$targetHead"
  Write-Host "SOURCE_TRACKED_PATHS=$($sourcePaths.Count)"
  if ($Push) {
    Invoke-Git push origin "HEAD:refs/heads/$targetBranch"
    Write-Host 'PUSHED_TO_MIGRATION_BRANCH=YES'
  } else {
    Write-Host 'PUSHED_TO_MIGRATION_BRANCH=NO (pass -Push after review)'
  }
} finally { Pop-Location }
