#requires -Version 7.0
<#
.SYNOPSIS
  Imports a pinned Orca upstream commit into an isolated FineCore subtree.
.DESCRIPTION
  Preserves upstream Git ancestry using git subtree (without --squash).
  Does not push, merge into main, rename Rust identifiers, or change remote configuration.
  Run from a clean clone of oigorbrito/finecore, on migration/orca-tree-import.
#>
[CmdletBinding()]
param(
    [string]$ExpectedBranch = 'migration/orca-tree-import',
    [string]$UpstreamUrl = 'https://github.com/okikorg/orca-harness.git',
    [string]$UpstreamCommit = '2a9fd399cfbbaad773e3cbed92bb6e24d7561f01',
    [string]$Prefix = 'vendor/orca-harness',
    [switch]$Execute
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
function Git([string[]]$Arguments) {
    & git @Arguments
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed with exit $LASTEXITCODE" }
}
function ReadGit([string[]]$Arguments) {
    $value = & git @Arguments
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed with exit $LASTEXITCODE" }
    return ($value | Out-String).Trim()
}
$root = ReadGit @('rev-parse','--show-toplevel')
Set-Location $root
$branch = ReadGit @('branch','--show-current')
if ($branch -ne $ExpectedBranch) { throw "Expected branch $ExpectedBranch, found $branch" }
if ((ReadGit @('status','--porcelain')).Length -ne 0) { throw 'Worktree is not clean' }
$origin = ReadGit @('remote','get-url','origin')
if ($origin -notmatch '(^|[:/])oigorbrito/finecore(\.git)?$') { throw "Unexpected origin: $origin" }
if (Test-Path $Prefix) { throw "Destination path already exists: $Prefix" }
$before = ReadGit @('rev-parse','HEAD')
Write-Host "FineCore HEAD: $before"
Write-Host "Orca commit: $UpstreamCommit"
Write-Host "Target: $Prefix"
if (-not $Execute) {
    Write-Host 'PLAN ONLY: no repository changes. Re-run with -Execute to import.'
    exit 0
}
# Fetch the exact immutable object, without modifying existing remote configuration.
Git @('fetch','--no-tags',$UpstreamUrl,$UpstreamCommit)
$resolved = ReadGit @('rev-parse','FETCH_HEAD^{commit}')
if ($resolved -ne $UpstreamCommit) { throw "Pinned upstream mismatch: $resolved" }
$upstreamTree = ReadGit @('rev-parse',"$UpstreamCommit^{tree}")
Git @('subtree','add',"--prefix=$Prefix",$UpstreamCommit,'-m',"chore(provenance): import Orca $UpstreamCommit unchanged")
$importedTree = ReadGit @('rev-parse',"HEAD:$Prefix")
if ($importedTree -ne $upstreamTree) { throw "TREE MISMATCH: imported $importedTree, expected $upstreamTree" }
$parent2 = ReadGit @('rev-parse','HEAD^2')
if ($parent2 -ne $UpstreamCommit) { throw "HISTORY MISMATCH: second parent $parent2" }
$parent1 = ReadGit @('rev-parse','HEAD^1')
if ($parent1 -ne $before) { throw "BASE MISMATCH: first parent $parent1" }
$changed = @(ReadGit @('diff','--name-only',"$before..HEAD") -split "`n" | Where-Object { $_ })
$outside = @($changed | Where-Object { -not $_.StartsWith("$Prefix/") })
if ($outside.Count -gt 0) { throw "Files outside subtree changed: $($outside -join ', ')" }
Write-Host "PASS: imported subtree SHA=$importedTree, ancestry preserved, changed files=$($changed.Count), no paths outside $Prefix"
Write-Host 'Local commit only. Inspect and push this branch manually; do not merge automatically.'
