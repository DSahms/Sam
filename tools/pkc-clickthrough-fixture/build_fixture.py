#!/usr/bin/env python3
"""Build a tiny synthetic PKC root for Sammy packaged click-through.

Copies the existing PKC gateway modules read-only. Does not modify the PKC git
repository. Output is local generated files, not Dave's corpus.
"""
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_PKC = Path(r"F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus")
SOURCE_REL = Path("03_approved_corpus/clickthrough/MAPLE_KETTLE.md")
STATEMENTS = [
    "The maple kettle is stored in the blue cupboard.",
    "I described the porch bell as cracked.",
    "The spare key may still be taped under the garden bench.",
]
SOURCE_BODY = "# Maple kettle click-through fixture\n\n" + "\n\n".join(STATEMENTS) + "\n"


def copy_gateway(pkc_root: Path, dest: Path) -> None:
    tools = dest / "tools"
    tools.mkdir(parents=True, exist_ok=True)
    for name in ("pkc_interview_gateway.py", "creative_source_disclosure.py"):
        src = pkc_root / "tools" / name
        if not src.is_file():
            raise SystemExit(f"PKC gateway file missing (repo unchanged): {src}")
        shutil.copy2(src, tools / name)


def write_source(dest: Path) -> tuple[str, str]:
    path = dest / SOURCE_REL
    path.parent.mkdir(parents=True, exist_ok=True)
    data = SOURCE_BODY.encode("utf-8")
    path.write_bytes(data)
    digest = hashlib.sha256(data).hexdigest()
    return f"SRC-SHA256-{digest}", digest


def write_index(dest: Path, source_id: str, digest: str) -> None:
    record = {
        "approval_state": "approved",
        "audience_authorization": "owner_authorized_private_use_only",
        "authority": "owner-approved",
        "citations": [
            {
                "available": True,
                "citation_status": "available",
                "record_ref": f"source:{source_id}",
                "repository_path": SOURCE_REL.as_posix(),
                "role": "evidence_object",
                "sha256": digest,
                "source_date": "2026-08-17",
                "source_id": source_id,
            }
        ],
        "confidence": 1.0,
        "content": SOURCE_BODY,
        "contradiction_state": False,
        "contradictions": [],
        "disclosure_authorization": "owner_authorized_private_use_only",
        "domain": None,
        "global_ref": f"source:{source_id}",
        "id": source_id,
        "provenance": {},
        "record_type": "source",
        "related_records": [],
        "relationships": [],
        "repository_path": SOURCE_REL.as_posix(),
        "sensitivity": "none",
        "sensitivity_status": "classified",
        "source_date": "2026-08-17",
        "source_references": [],
        "source_sha256": digest,
        "supersession": {
            "state": "not_recorded",
            "superseded_by": [],
            "supersedes": [],
        },
        "temporality": None,
        "title": "Maple kettle click-through fixture",
        "trust_state": "evidence_object",
        "unresolved": [],
        "unresolved_state": False,
    }
    index = {
        "average_document_length": len(SOURCE_BODY.split()),
        "document_count": 1,
        "document_frequency": {},
        "documents": [{"length": len(SOURCE_BODY.split()), "record": record, "terms": []}],
        "export_fingerprint": digest,
        "index_version": "0.1",
        "ranking": {},
    }
    out = dest / "06_exports/generated/pkc-v0.1.index.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(index, indent=2), encoding="utf-8")


def write_register(dest: Path, source_id: str) -> None:
    register = {
        "default_decision": "deny",
        "policies": [
            {
                "authoritative_source_ids": [source_id],
                "candidate_source_ids": [],
                "effective": {
                    "policy_version": "0.1",
                    "valid_from": "2026-08-17",
                    "valid_until": None,
                },
                "identity_status": "verified",
                "notes": [
                    "Synthetic Sammy click-through fixture. Not owner autobiographical corpus."
                ],
                "policy_id": "CSDP-MAPLE-KETTLE",
                "policy_provenance": {
                    "decision_authority": "clickthrough-fixture",
                    "decision_date": "2026-08-17",
                    "source_references": ["tools/pkc-clickthrough-fixture/README.md"],
                },
                "policy_status": "approved",
                "review_state": "approved",
                "rules": [
                    {
                        "allowed_content_level": "full_text",
                        "consent_requirement": "standing_authorization",
                        "consumer_classes": ["sammy"],
                        "excerpt_limit_chars": None,
                        "full_text_permission": "allowed",
                        "provenance_required": True,
                        "purposes": ["personal_consigliere"],
                        "realms": ["private_autobiographical_interview"],
                        "recipient_classes": ["owner_dave"],
                        "rule_id": "RULE-SAMMY-PRIVATE-CONSULIERE",
                        "sensitivity_requirements": ["owner_authorized_private_use_only"],
                    }
                ],
                "sensitivity_status": "resolved",
                "title": "Maple kettle click-through fixture",
                "work_id": "WORK-MAPLE-KETTLE",
            }
        ],
        "register_id": "CSDR-CLICKTHROUGH-MAPLE-KETTLE",
        "register_status": "approved",
        "schema_version": "0.1.0",
    }
    out = dest / "06_exports/creative-source-disclosure-register-v0.1.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(register, indent=2), encoding="utf-8")


def main() -> int:
    pkc_root = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PKC
    dest = Path(sys.argv[2]) if len(sys.argv) > 2 else HERE / "generated"
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    copy_gateway(pkc_root, dest)
    source_id, digest = write_source(dest)
    write_index(dest, source_id, digest)
    write_register(dest, source_id)
    manifest = {
        "pkc_root": str(dest),
        "source_id": source_id,
        "source_sha256": digest,
        "question": "Where did I store the maple kettle?",
        "statements": STATEMENTS,
        "pkc_git_commit_expected": "b0234ccfa52096aaaa5290b2b7253f9ced136853",
    }
    (dest / "FIXTURE.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
