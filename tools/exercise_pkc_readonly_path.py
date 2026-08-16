#!/usr/bin/env python3
"""Exercise Sammy's read-only PKC path without enabling the production gate.

Uses the PKC Reference Client bridge with consumer_application=sammy.
Prints hashes and booleans only — never source bodies or secrets.
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

BRIDGE = Path(r"D:\dev\StoryKeeper\tools\storykeeper_pkc_bridge.py")
PKC_ROOT = Path(r"F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus")
SEVEN = "SRC-SHA256-9c53f4f121815dfd736e0377fafade3e649b2571f8858eebc8f1a92db2b0ec7f"
KOBOLD = "http://127.0.0.1:5001/v1/chat/completions"

FORBIDDEN = (
    "CSDP-",
    "RULE-STORYKEEPER",
    "RULE-SAMMY",
    "evidence_bookkeeping",
    "authorization_denied",
    "canonical_hash",
    "record_trust_state",
)


def main() -> int:
    sys.path.insert(0, str(BRIDGE.parent))
    from storykeeper_pkc_bridge import handle_request

    request = {
        "bridge_version": "1.0.0",
        "request_id": "SAMMY-LIVE-READONLY-001",
        "operation": "protected_retrieval",
        "question": "What do you remember about moving as a child?",
        "chapter_context": "Personal question — Sammy read-only PKC consult",
        "consumer_application": "sammy",
        "recipient_class": "owner_dave",
        "realm": "private_autobiographical_interview",
        "purpose": "personal_consigliere",
        "disclosure_mode": "private_local",
        "requested_content_level": "full_text",
        "consent_state": "standing_authorization",
        "source_id": SEVEN,
    }

    os.chdir(BRIDGE.parent)
    result = handle_request(request, pkc_root=PKC_ROOT)
    payload = result.get("payload") or {}
    answer = payload.get("natural_answer") or ""
    leaks = [marker for marker in FORBIDDEN if marker in answer]
    report = {
        "ok": bool(result.get("ok")),
        "available": bool(result.get("available")),
        "authorized": bool(payload.get("authorized")),
        "canonical_hash_verified": bool(payload.get("canonical_hash_verified")),
        "protected_content_materialized": bool(
            payload.get("protected_content_materialized")
        ),
        "payload_sha256": hashlib.sha256(answer.encode("utf-8")).hexdigest()
        if answer
        else "",
        "payload_chars": len(answer),
        "bookkeeping_in_payload": leaks,
        "kobold_used": False,
        "kobold_crossed_cloud": False,
        "durable_memory_write": False,
    }
    if leaks or not report["authorized"] or not answer:
        json.dump(report, sys.stdout, indent=2)
        print()
        return 2

    try:
        body = json.dumps(
            {
                "model": "koboldcpp",
                "max_tokens": 80,
                "messages": [
                    {
                        "role": "system",
                        "content": (
                            "Distinguish source-backed facts, user testimony, and inference. "
                            "Do not invent sensory scene. Never mention PKC bookkeeping. "
                            f"SOURCE-BACKED FACTS:\n{answer[:800]}"
                        ),
                    },
                    {
                        "role": "user",
                        "content": "What is supported about the childhood moves?",
                    },
                ],
            }
        ).encode("utf-8")
        req = urllib.request.Request(
            KOBOLD,
            data=body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=90) as resp:
            raw = json.loads(resp.read().decode("utf-8"))
        content = raw["choices"][0]["message"]["content"]
        report["kobold_used"] = True
        report["kobold_response_sha256"] = hashlib.sha256(
            content.encode("utf-8")
        ).hexdigest()
        report["kobold_leaks_bookkeeping"] = [m for m in FORBIDDEN if m in content]
    except (
        urllib.error.URLError,
        TimeoutError,
        KeyError,
        json.JSONDecodeError,
        OSError,
    ) as exc:
        report["kobold_used"] = False
        report["kobold_skip_reason"] = type(exc).__name__
    json.dump(report, sys.stdout, indent=2)
    print()
    return 0 if report["authorized"] and not report["bookkeeping_in_payload"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
