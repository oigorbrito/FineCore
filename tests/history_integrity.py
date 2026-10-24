"""Verify the independent Git root and source-only distribution."""
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]

def git(*args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

head = git('rev-parse', 'HEAD')
commits = git('rev-list', '--count', 'HEAD')
parents = git('rev-list', '--parents', '-n', '1', 'HEAD').split()
assert int(commits) >= 1, 'Missing history'
roots = git('rev-list', '--max-parents=0', 'HEAD').splitlines()
assert len(roots) == 1, f'Expected one root: {roots}'
assert len(git('rev-list', '--parents', '-n', '1', roots[0]).split()) == 1, 'Root has parents'
assert all(not p.startswith('vendor/') for p in git('ls-tree', '-r', '--name-only', 'HEAD').splitlines()), 'Unexpected donor vendor tree'
print(f'PASS: independent root {roots[0]}, 0 root parents, {commits} local commits')
