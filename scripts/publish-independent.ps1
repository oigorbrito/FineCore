# Publishes a genuinely independent Git root as a NEW remote branch.
# Never rewrites main or an existing branch.
[CmdletBinding()]
param(
    [string]$Repository = 'https://github.com/oigorbrito/finecore.git',
    [string]$Branch = 'bootstrap/finecore-independent'
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
function Git([string[]]$Arguments) {
    & git -C $root @Arguments
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed ($LASTEXITCODE)" }
}
if ($Branch -eq 'main' -or $Branch -eq 'master') { throw 'Refusing to publish to a default branch.' }
Git -Arguments @('check-ref-format', '--branch', $Branch)
$dirty = & git -C $root status --porcelain
if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Local worktree must be clean.' }
$roots = @(& git -C $root rev-list --max-parents=0 HEAD)
if ($LASTEXITCODE -ne 0 -or $roots.Count -ne 1) { throw 'Expected exactly one root commit.' }
$parents = @((& git -C $root rev-list --parents -n 1 $roots[0]) -split ' ')
if ($parents.Count -ne 1) { throw 'Root commit has parents.' }
$existing = & git ls-remote --exit-code --heads $Repository "refs/heads/$Branch"
if ($LASTEXITCODE -eq 0) { throw "Remote branch $Branch already exists; refusing overwrite." }
if ($LASTEXITCODE -ne 2) { throw 'Cannot verify remote branch absence; refusing push.' }
$sha = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Unable to read HEAD.' }
& git -C $root push --porcelain $Repository "${sha}:refs/heads/$Branch"
if ($LASTEXITCODE -ne 0) { throw 'Push failed; remote state must be inspected before retry.' }
Write-Host "Published independent history: $Branch at $sha"
