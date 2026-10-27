#!/usr/bin/env python3
"""Bounded independent Pi upstream monitoring for FineCore.

Only source GETs; at most one destination tracking issue is created/updated.
All data are evidence or triage hints, never code imports or benchmark claims.
"""
from __future__ import annotations

import datetime as dt
import hashlib
import html
import json
import os
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

API = "https://api.github.com"
SOURCE = "earendil-works/pi"
TARGET = os.environ.get("TRACKING_REPO", "")
TOKEN = os.environ.get("GH_TOKEN", "")
DRY_RUN = os.environ.get("PI_TRACKER_DRY_RUN") == "1"
MARKER_START = "<!-- finecore-pi-monitor:"
MARKER_END = " -->"
MAX_PAGES = 20
WINDOW_DAYS = 14
MAX_BODY_BYTES = 62000
REPORT_PATH = Path(os.environ.get("PI_TRACKER_REPORT", "pi-upstream-evolution.json"))
TOPIC = {
    "auth": ("oauth", "login", "credential", "token", "auth", "keychain"),
    "model_provider": ("model", "provider", "stream", "anthropic", "openai", "gemini", "xai"),
    "agent": ("agent", "tool", "mcp", "session", "skill", "context", "compaction"),
    "performance": ("perform", "speed", "cache", "latency", "benchmark", "token", "cost", "memory"),
    "security": ("secur", "vulnerab", "advisory", "permission", "sandbox", "injection"),
    "tui": ("tui", "terminal", "keyboard", "render", "input"),
}


def api(method: str, path: str, payload: dict | None = None):
    if method != "GET":
        if not TARGET or not TOKEN:
            raise RuntimeError("Destination writes require GH_TOKEN and TRACKING_REPO")
        if not path.startswith(f"/repos/{TARGET}/issues"):
            raise RuntimeError("Only FineCore issue writes are allowed")
    url = API + path
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "finecore-pi-monitor/1",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if TOKEN:
        headers["Authorization"] = "Bearer " + TOKEN
    data = json.dumps(payload).encode() if payload is not None else None
    if data is not None:
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=35) as response:
            return json.loads(response.read())
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"GitHub {method} {path.split('?')[0]} HTTP {error.code}") from error


def collection(path: str, *, cutoff: str | None = None):
    """A page limit or ordering anomaly is a failure, not a complete scan."""
    previous = None
    separator = "&" if "?" in path else "?"
    for page in range(1, MAX_PAGES + 1):
        items = api("GET", f"{path}{separator}per_page=100&page={page}")
        if not isinstance(items, list):
            raise RuntimeError("Expected paginated GitHub list")
        for item in items:
            updated = item.get("updated_at") or ""
            if cutoff is not None and previous and updated > previous:
                raise RuntimeError("Upstream updated_at ordering changed")
            if updated:
                previous = updated
            if cutoff and updated and updated < cutoff:
                return
            yield item
        if len(items) < 100:
            return
    raise RuntimeError(f"Pagination exceeded {MAX_PAGES} pages: {path}")


def search_recent(kind: str, cutoff: str):
    """Exhaustive GitHub Search window, no ordering assumptions.

    Search caps result access at 1,000 records. Reject over-cap or incomplete
    results instead of claiming every upstream change was seen.
    """
    if kind not in {"pr", "issue"}:
        raise ValueError("unknown GitHub search kind")
    term = f"repo:{SOURCE} is:{kind} updated:>={cutoff[:10]}"
    query = urllib.parse.urlencode({"q": term, "sort": "updated", "order": "desc"})
    collected = []
    expected = None
    for page in range(1, 11):
        data = api("GET", f"/search/issues?{query}&per_page=100&page={page}")
        if not isinstance(data, dict) or data.get("incomplete_results"):
            raise RuntimeError("Incomplete GitHub Search result")
        total = data.get("total_count")
        if not isinstance(total, int) or total > 900:
            raise RuntimeError(f"GitHub Search result exceeds safe cap for {kind}: {total}")
        if expected is None:
            expected = total
        if expected != total:
            raise RuntimeError("GitHub Search inventory changed while paginating")
        items = data.get("items")
        if not isinstance(items, list):
            raise RuntimeError("GitHub Search items missing")
        collected.extend(items)
        if len(collected) >= expected:
            break
        if len(items) < 100:
            raise RuntimeError("GitHub Search pagination stopped early")
    numbers = {i["number"] for i in collected}
    if len(collected) != expected or len(numbers) != expected:
        raise RuntimeError(f"Incomplete or duplicated {kind} search inventory")
    # In production, the Search endpoint once returned normal issues to an
    # is:pr query, creating an invalid PR count. Never trust only total_count.
    # This is a fail-closed source type gate, not a cosmetic filter: dropping
    # mismatched records would wrongly claim a complete scan.
    bad_type = [
        i["number"] for i in collected
        if ("pull_request" in i) != (kind == "pr")
    ]
    if bad_type:
        raise RuntimeError(
            f"GitHub Search returned {len(bad_type)} wrong-type records for {kind}"
        )
    expected_segment = "/pull/" if kind == "pr" else "/issues/"
    if any(expected_segment not in i.get("html_url", "") for i in collected):
        raise RuntimeError(f"GitHub Search has invalid {kind} URLs")
    return collected


def source_snapshot(now: dt.datetime):
    cutoff = (now - dt.timedelta(days=WINDOW_DAYS)).strftime("%Y-%m-%dT%H:%M:%SZ")
    repository = api("GET", f"/repos/{SOURCE}")
    default = repository["default_branch"]
    commit = api("GET", f"/repos/{SOURCE}/commits/{urllib.parse.quote(default, safe='')}")
    if not re.fullmatch(r"[a-f0-9]{40}", commit["sha"]):
        raise RuntimeError("Invalid upstream main SHA")
    search_prs = search_recent("pr", cutoff)
    # Search issue records omit head.sha. The entire open-PR collection
    # provides the actual SHA for each active candidate.
    recent_prs = [
        {
            "number": issue["number"], "state": issue["state"],
            "updated_at": issue["updated_at"], "title": issue["title"],
            "html_url": issue["html_url"], "head": {"sha": None},
            "merged_at": (issue.get("pull_request") or {}).get("merged_at"),
        }
        for issue in search_prs
    ]
    open_prs = list(collection(f"/repos/{SOURCE}/pulls?state=open&sort=updated&direction=desc"))
    recent_issues = search_recent("issue", cutoff)
    releases = list(collection(f"/repos/{SOURCE}/releases"))
    by_pr_number = {pr["number"]: pr for pr in recent_prs}
    for pr in open_prs:
        by_pr_number[pr["number"]] = pr
    prs = sorted(by_pr_number.values(), key=lambda p: (p["updated_at"], p["number"]), reverse=True)
    issue_ids = [
        [i["number"], i["state"], i["updated_at"]] for i in recent_issues
    ]
    pr_ids = [
        [p["number"], p["state"], p["updated_at"], p["head"]["sha"], p.get("merged_at")]
        for p in prs
    ]
    digest = lambda data: hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()
    latest_release = next((r for r in releases if not r.get("draft") and r.get("published_at")), None)
    state = {
        "default_branch": default,
        "head": commit["sha"],
        "latest_release": (
            {"tag": latest_release["tag_name"], "published_at": latest_release["published_at"]}
            if latest_release else None
        ),
        "open_pr_count": len(open_prs),
        "recent_pr_count": len(recent_prs),
        "recent_issue_count": len(recent_issues),
        "pr_digest": digest(pr_ids),
        "issue_digest": digest(issue_ids),
        "release_digest": digest([[r.get("id"), r.get("tag_name"), r.get("updated_at")] for r in releases]),
        # Bounded history makes changes auditable in the tracker issue while
        # full records remain in the Actions artifact.
        "recent_pr_ids": pr_ids[:70],
        "recent_issue_ids": issue_ids[:70],
    }
    candidates = []
    for pr in prs:
        title = " ".join((pr.get("title") or "").split())
        labels = [topic for topic, words in TOPIC.items()
                  if any(word in title.lower() for word in words)]
        if labels:
            candidates.append({
                "pr": pr["number"], "title": title[:190],
                "head": pr["head"]["sha"], "state": pr["state"],
                "merged_at": pr.get("merged_at"),
                "updated_at": pr["updated_at"], "topics_title_heuristic": labels,
                "url": pr["html_url"],
            })
    return state, {
        "schema_version": 1,
        "source": SOURCE,
        "target": TARGET,
        "source_head": commit["sha"],
        "cutoff": cutoff,
        "counts": {"open_prs": len(open_prs), "recent_prs": len(recent_prs),
                   "recent_issues": len(recent_issues), "releases": len(releases)},
        "candidates": candidates,
        "latest_release": state["latest_release"],
        "full_recent_prs": pr_ids,
        "full_recent_issues": issue_ids,
        "policy": "metadata/heuristic only; no performance improvement claimed",
    }


def marker(state: dict) -> str:
    encoded = json.dumps(state, sort_keys=True, separators=(",", ":")).replace("-->", r"\u002d\u002d\u003e")
    return MARKER_START + encoded + MARKER_END


def parse_marker(body: str):
    start = body.find(MARKER_START)
    if start < 0:
        return None
    if body.count(MARKER_START) != 1:
        raise ValueError("Duplicate Pi monitor marker")
    payload = body[start + len(MARKER_START):]
    state, offset = json.JSONDecoder().raw_decode(payload)
    if not isinstance(state, dict) or not payload[offset:].startswith(MARKER_END):
        raise ValueError("Malformed Pi monitor marker")
    return state


def tracked_issue():
    found = None
    for issue in collection(f"/repos/{TARGET}/issues?state=all"):
        if "pull_request" in issue:
            continue
        state = parse_marker(issue.get("body") or "")
        if state is not None:
            if found is not None:
                raise RuntimeError("Duplicate FineCore Pi monitor issues")
            found = issue
    return found


def describe(state: dict, report: dict) -> str:
    top = report["candidates"][:18]
    lines = [
        "Monitoramento independente, somente metadados. Nenhum código importado, merge ou ganho de benchmark alegado.",
        "",
        f"Pi main: https://github.com/{SOURCE}/commit/{state['head']}",
        f"Release mais recente: {state['latest_release']['tag'] if state['latest_release'] else 'nenhuma'}",
        f"PRs abertas: {state['open_pr_count']}; PRs recentes: {state['recent_pr_count']}; issues recentes: {state['recent_issue_count']}.",
        "Janela de observação: 14 dias. As PRs abertas antigas também são incluídas.",
        "",
        "## Candidatos para auditoria (classificação de título, NÃO benchmark)",
    ]
    for pr in top:
        title = html.escape(pr["title"]).replace("<!--", "&lt;!--")
        topics = ", ".join(pr["topics_title_heuristic"])
        head = pr["head"][:12] if pr["head"] else "não disponível no Search API"
        lines.append(f"- [Pi #{pr['pr']}]({pr['url']}) [{topics}]: {title} (head {head})")
    lines += [
        "", "Antes de importar ou concluir vantagem: comparar contrato, qualidade em tarefas",
        "pareadas, custo/tarefa correta, tokens, tempo e regressões. Revisão e merge manuais.",
        "", "## Estado técnico do monitor", marker(state),
    ]
    body = "\n".join(lines)
    if len(body.encode("utf-8")) > MAX_BODY_BYTES:
        raise RuntimeError("Monitor issue would exceed safe size limit")
    return body


def sync(state: dict, report: dict) -> dict:
    if TARGET != "oigorbrito/finecore":
        raise RuntimeError("Wrong destination; expected oigorbrito/finecore")
    previous = tracked_issue()
    old_state = parse_marker(previous.get("body") or "") if previous else None
    report["delta"] = {
        "head_changed": old_state is not None and old_state.get("head") != state["head"],
        "prs_changed": old_state is not None and old_state.get("pr_digest") != state["pr_digest"],
        "issues_changed": old_state is not None and old_state.get("issue_digest") != state["issue_digest"],
        "release_changed": old_state is not None and old_state.get("release_digest") != state["release_digest"],
        "baseline": old_state is None,
    }
    if old_state == state:
        return {"action": "unchanged", "issue": previous["number"]}
    body = describe(state, report)
    title = "[upstream:earendil-works/pi] Monitor de evolução e triagem FineCore"
    if previous:
        begin = previous["body"].index(MARKER_START)
        end = begin + len(MARKER_START)
        _, offset = json.JSONDecoder().raw_decode(previous["body"][end:])
        end += offset + len(MARKER_END)
        # Preserve human annotations added after the managed snapshot.
        suffix = previous["body"][end:].strip()
        if suffix:
            body += "\n\n" + suffix
        updated = api("PATCH", f"/repos/{TARGET}/issues/{previous['number']}",
                      {"title": title, "body": body})
        return {"action": "updated", "issue": updated["number"]}
    created = api("POST", f"/repos/{TARGET}/issues",
                  {"title": title, "body": body})
    return {"action": "created", "issue": created["number"]}


def main():
    if not TARGET or (not TOKEN and not DRY_RUN):
        raise RuntimeError("Missing tracking target or read token")
    now = dt.datetime.now(dt.timezone.utc)
    state, report = source_snapshot(now)
    report["sync"] = ({"action": "dry_run", "issue": None} if DRY_RUN else sync(state, report))
    if DRY_RUN:
        report["delta"] = {"status": "no destination writes or checkpoint comparisons in dry run"}
    REPORT_PATH.write_text(json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
                           encoding="utf-8")
    print(json.dumps({"source": SOURCE, "head": state["head"], "counts": report["counts"],
                      "sync": report["sync"], "delta": report["delta"]}, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"PI_TRACKER_FAILED: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
