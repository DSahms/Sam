#!/usr/bin/env python3
"""Diagnostic tool for Sammy's read-only PKC path.

This is not the owner experience. Daily use is Settings + Chat in Sammy.
This script checks the same authorized bridge the application uses.

Modes:
  health       path + authorization probe (no corpus text printed)
  auth         authorization boolean only
  retrieval    authorized retrieval hashes/counts
  sanitize     confirm bookkeeping is absent from the conversational payload
  local        optional KoboldCpp grounded call
  all          default: health + retrieval + local if reachable

Prints JSON only — never source bodies or secrets.
"""
from __future__ import annotations

import argparse
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


def _payload(question: str) -> dict:
    sys.path.insert(0, str(BRIDGE.parent))
    from storykeeper_pkc_bridge import handle_request

    request = {
        "bridge_version": "1.0.0",
        "request_id": "SAMMY-DIAGNOSTIC-001",
        "operation": "protected_retrieval",
        "question": question,
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
    return handle_request(request, pkc_root=PKC_ROOT)


def _health() -> dict:
    python_ok = True
    return {
        "mode": "health",
        "python_ok": python_ok,
        "bridge_ok": BRIDGE.is_file(),
        "root_ok": PKC_ROOT.is_dir(),
        "consumer": "sammy",
        "purpose": "personal_consigliere",
        "local_only": True,
    }


def _retrieval(question: str) -> dict:
    result = _payload(question)
    payload = result.get("payload") or {}
    answer = payload.get("natural_answer") or ""
    leaks = [marker for marker in FORBIDDEN if marker in answer]
    return {
        "mode": "retrieval",
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
        "durable_memory_write": False,
    }


def _kobold(answer: str) -> dict:
    report = {"kobold_used": False, "kobold_crossed_cloud": False}
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
        report["kobold_skip_reason"] = type(exc).__name__
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description="Sammy PKC diagnostic (not the UI)")
    parser.add_argument(
        "--mode",
        choices=["health", "auth", "retrieval", "sanitize", "local", "all"],
        default="all",
    )
    parser.add_argument(
        "--question",
        default="What do you remember about moving as a child?",
    )
    args = parser.parse_args()

    if not BRIDGE.is_file() or not PKC_ROOT.is_dir():
        json.dump(
            {
                "ok": False,
                "state": "misconfigured",
                "bridge_ok": BRIDGE.is_file(),
                "root_ok": PKC_ROOT.is_dir(),
            },
            sys.stdout,
            indent=2,
        )
        print()
        return 2

    health = _health()
    if args.mode == "health":
        json.dump(health, sys.stdout, indent=2)
        print()
        return 0 if health["bridge_ok"] and health["root_ok"] else 2

    retrieved = _retrieval(args.question)
    if args.mode == "auth":
        json.dump(
            {
                "mode": "auth",
                "authorized": retrieved["authorized"],
                "available": retrieved["available"],
                "consumer": "sammy",
                "purpose": "personal_consigliere",
            },
            sys.stdout,
            indent=2,
        )
        print()
        return 0 if retrieved["authorized"] else 2

    if args.mode == "sanitize":
        json.dump(
            {
                "mode": "sanitize",
                "payload_chars": retrieved["payload_chars"],
                "payload_sha256": retrieved["payload_sha256"],
                "bookkeeping_in_payload": retrieved["bookkeeping_in_payload"],
                "model_safe": not retrieved["bookkeeping_in_payload"]
                and retrieved["authorized"],
            },
            sys.stdout,
            indent=2,
        )
        print()
        return 0 if retrieved["authorized"] and not retrieved["bookkeeping_in_payload"] else 2

    if args.mode == "retrieval":
        json.dump(retrieved, sys.stdout, indent=2)
        print()
        return 0 if retrieved["authorized"] and not retrieved["bookkeeping_in_payload"] else 2

    report = {**health, **retrieved}
    if args.mode in ("local", "all"):
        answer_len = retrieved.get("payload_chars") or 0
        if retrieved["authorized"] and answer_len:
            # Re-fetch only for the local provider test; never print the body.
            result = _payload(args.question)
            answer = (result.get("payload") or {}).get("natural_answer") or ""
            report.update(_kobold(answer))
        else:
            report["kobold_used"] = False
            report["kobold_skip_reason"] = "no_authorized_payload"

    json.dump(report, sys.stdout, indent=2)
    print()
    if not report.get("authorized") or report.get("bookkeeping_in_payload"):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
