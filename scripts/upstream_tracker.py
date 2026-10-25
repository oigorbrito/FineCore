#!/usr/bin/env python3
"""Independent upstream mirror: GitHub issues, PR updates, PR commits, and default branch.
Upstream is strictly read-only; destination receives tracker issues only.
"""
import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

API = "https://api.github.com"
MARKER = re.compile(r"<!-- finecore-upstream:([^ ]+) state:(\{.*?\}) -->")
TOKEN = os.environ.get("GH_TOKEN", "")
DEST = os.environ.get("TRACKING_REPO", "")
UPSTREAMS = [x.strip() for x in os.environ.get("UPSTREAM_REPOSITORIES", "okikorg/orca-harness").split(",") if x.strip()]
MAX_PAGES = 30


def request(method, path, payload=None):
    url = API + path
    data = None if payload is None else json.dumps(payload).encode()
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "finecore-upstream-tracker/1", "X-GitHub-Api-Version": "2022-11-28"}
    if TOKEN:
        headers["Authorization"] = "Bearer " + TOKEN
    if data is not None:
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=35) as res:
                raw = res.read()
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as exc:
            if exc.code in (403, 429, 500, 502, 503) and attempt < 3:
                time.sleep(2 ** attempt)
                continue
            raise RuntimeError(f"GitHub {method} {path}: HTTP {exc.code}") from exc
    raise RuntimeError(f"GitHub request failed: {method} {path}")


def pages(path):
    sep = "&" if "?" in path else "?"
    for page in range(1, MAX_PAGES + 1):
        items = request("GET", f"{path}{sep}per_page=100&page={page}")
        if not isinstance(items, list):
            raise RuntimeError("Expected list: " + path)
        yield from items
        if len(items) < 100:
            return
    raise RuntimeError("Pagination cap reached (fail closed): " + path)


def marker(key, state):
    return "<!-- finecore-upstream:" + key + " state:" + json.dumps(state, sort_keys=True, separators=(",", ":")) + " -->"


def parse(body):
    match = MARKER.search(body or "")
    if not match:
        return None
    return match.group(1), json.loads(match.group(2))


def snapshot(upstream, kind, item, previous=None):
    state = {"updated_at": item.get("updated_at"), "state": item.get("state"), "title": item.get("title")}
    if kind == "pr":
        pr = request("GET", f"/repos/{upstream}/pulls/{item['number']}")
        cached = previous or {}
        if cached.get("head") == pr["head"]["sha"] and cached.get("base") == pr["base"]["sha"] and "commits_digest" in cached:
            state.update({"head": pr["head"]["sha"], "base": pr["base"]["sha"],
                          "draft": pr.get("draft", False), "merged_at": pr.get("merged_at"),
                          "commit_count": cached["commit_count"],
                          "commits_digest": cached["commits_digest"],
                          "last_commit": cached.get("last_commit")})
            return state
        commits = list(pages(f"/repos/{upstream}/pulls/{item['number']}/commits"))
        state.update({"head": pr["head"]["sha"], "base": pr["base"]["sha"],
                      "draft": pr.get("draft", False), "merged_at": pr.get("merged_at"),
                      "commit_count": len(commits),
                      "commits_digest": hashlib.sha256("\n".join(c["sha"] for c in commits).encode()).hexdigest(),
                      "last_commit": commits[-1]["sha"] if commits else None})
    return state


def sync_one(upstream, kind, item, indexed):
    key = f"{upstream}:{kind}:{item['number']}"
    previous = indexed.get(key)
    state = snapshot(upstream, kind, item, previous[1] if previous else None)
    if previous and previous[1] == state:
        return "unchanged"
    title = f"[upstream:{upstream}] {'PR' if kind == 'pr' else 'Issue'} #{item['number']}: {item['title']}"[:250]
    url = item["html_url"]
    body = (f"Source (read-only): {url}\n\n"
            f"Type: {kind}; upstream: {upstream}\n\n"
            "This is an independent tracking record, **not** an import or merge approval. "
            "No upstream branch, issue or pull request is modified.\n\n"
            f"Last observed snapshot:\n\x60\x60\x60json\n{json.dumps(state, indent=2, sort_keys=True)}\n\x60\x60\x60\n\n"
            + marker(key, state))
    if previous:
        number, old = previous
        # Update the issue snapshot directly. No comment is posted: retries after a
        # successful PATCH remain idempotent, even if the process crashes mid-run.
        request("PATCH", f"/repos/{DEST}/issues/{number}", {"title": title, "body": body})
        indexed[key] = (number, state)
        return "updated"
    created = request("POST", f"/repos/{DEST}/issues", {"title": title, "body": body})
    indexed[key] = (created["number"], state)
    return "created"


def sync_branch(upstream, indexed):
    repo = request("GET", f"/repos/{upstream}")
    default = repo["default_branch"]
    commit = request("GET", f"/repos/{upstream}/commits/{urllib.parse.quote(default, safe='')}")
    key = f"{upstream}:default-branch"
    state = {"branch": default, "sha": commit["sha"], "url": commit["html_url"]}
    previous = indexed.get(key)
    if previous and previous[1] == state:
        return "unchanged"
    body = f"Upstream default branch (read-only): {commit['html_url']}\n\n" + marker(key, state)
    title = f"[upstream:{upstream}] Default branch updates"
    if previous:
        number, old = previous
        request("PATCH", f"/repos/{DEST}/issues/{number}", {"title": title, "body": body})
        indexed[key] = (number, state)
        return "updated"
    issue = request("POST", f"/repos/{DEST}/issues", {"title": title, "body": body})
    indexed[key] = (issue["number"], state)
    return "created"



def sync_record(upstream, kind, identifier, url, state, indexed):
    """Upsert a stable snapshot into a FineCore issue, without duplicate comments."""
    key = f"{upstream}:{kind}:{identifier}"
    previous = indexed.get(key)
    if previous and previous[1] == state:
        return "unchanged"
    title = f"[upstream:{upstream}] {kind.title()} {identifier}"[:250]
    body = (f"Source (read-only): {url}\n\n"
            f"Type: {kind}; upstream: {upstream}\n\n"
            "This is a tracking record, not a code import or merge approval.\n\n"
            f"Last observed snapshot:\n```json\n{json.dumps(state, indent=2, sort_keys=True)}\n```\n\n"
            + marker(key, state))
    if previous:
        number, _ = previous
        request("PATCH", f"/repos/{DEST}/issues/{number}", {"title": title, "body": body})
        result = "updated"
    else:
        issue = request("POST", f"/repos/{DEST}/issues", {"title": title, "body": body})
        number = issue["number"]
        result = "created"
    indexed[key] = (number, state)
    return result


def sync_releases_and_tags(upstream, indexed, stats):
    """Track published releases and tags, including deletion and restoration."""
    collections = (
        ("release", f"/repos/{upstream}/releases"),
        ("tag", f"/repos/{upstream}/tags"),
    )
    for kind, path in collections:
        seen = set()
        for item in pages(path):
            if kind == "release":
                identifier = str(item["id"])
                url = item["html_url"]
                state = {
                    "tag_name": item["tag_name"],
                    "name": item.get("name"),
                    "draft": bool(item.get("draft")),
                    "prerelease": bool(item.get("prerelease")),
                    "published_at": item.get("published_at"),
                    "updated_at": item.get("updated_at"),
                    "target_commitish": item.get("target_commitish"),
                    "deleted": False,
                }
            else:
                identifier = item["name"]
                url = f"https://github.com/{upstream}/releases/tag/{urllib.parse.quote(identifier, safe='')}"
                state = {"sha": item["commit"]["sha"], "deleted": False}
            key = f"{upstream}:{kind}:{identifier}"
            seen.add(key)
            stats[sync_record(upstream, kind, identifier, url, state, indexed)] += 1
        # Pagination must complete before declaring an existing record absent.
        prefix = f"{upstream}:{kind}:"
        for key, (_, old) in list(indexed.items()):
            if key.startswith(prefix) and key not in seen and not old.get("deleted"):
                identifier = key[len(prefix):]
                state = dict(old, deleted=True)
                url = f"https://github.com/{upstream}"
                stats[sync_record(upstream, kind, identifier, url, state, indexed)] += 1


def main():
    if not TOKEN or not DEST:
        raise RuntimeError("GH_TOKEN and TRACKING_REPO are required")
    if not UPSTREAMS or len(UPSTREAMS) > 10:
        raise RuntimeError("Expected 1-10 upstream repositories")
    if DEST in UPSTREAMS:
        raise RuntimeError("Destination cannot track itself")
    indexed = {}
    for issue in pages(f"/repos/{DEST}/issues?state=all"):
        if "pull_request" in issue:
            continue
        parsed = parse(issue.get("body"))
        if parsed:
            key, state = parsed
            if key in indexed:
                raise RuntimeError("Duplicate tracker marker: " + key)
            indexed[key] = (issue["number"], state)
    stats = {"created": 0, "updated": 0, "unchanged": 0}
    for upstream in UPSTREAMS:
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", upstream):
            raise RuntimeError("Invalid upstream: " + upstream)
        # Fetch all issues (including PR-backed issue objects) to retain closed state changes.
        for item in pages(f"/repos/{upstream}/issues?state=all&sort=updated&direction=desc"):
            kind = "pr" if "pull_request" in item else "issue"
            stats[sync_one(upstream, kind, item, indexed)] += 1
        stats[sync_branch(upstream, indexed)] += 1
        sync_releases_and_tags(upstream, indexed, stats)
    print(json.dumps({"destination": DEST, "upstreams": UPSTREAMS, "stats": stats}, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"TRACKER_FAILED: {exc}", file=sys.stderr)
        sys.exit(1)
