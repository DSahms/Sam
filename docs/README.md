# Sammy documentation

Sammy is easiest to understand as three cooperating systems: an encrypted vault,
an owner-controlled Personal Knowledge Corpus, and a replaceable model provider.
This documentation describes the implemented `0.1.0` release candidate.

## Choose a path

| I want to… | Start here |
| --- | --- |
| Install and talk to a local model | [Quick start](getting-started/quick-start.md) |
| Understand the product model | [Personal Knowledge Corpus](concepts/personal-knowledge-corpus.md) |
| Use a specific screen | [User guide](#user-guide) |
| Diagnose a failure | [Troubleshooting](TROUBLESHOOTING.md) |
| Review security | [Security model](security/security-model.md) |
| Build or change Sammy | [Development setup](development/setup.md) |
| Prepare a release | [Release process](development/release-process.md) |

New to localhost, SQLCipher, OCR, or vector indexes? The
[glossary](reference/glossary.md) explains them without requiring architecture knowledge.

## Getting started

- [Installation](getting-started/installation.md)
- [First run](getting-started/first-run.md)
- [Quick start](getting-started/quick-start.md)
- [KoboldCpp](getting-started/koboldcpp.md)
- [External personal knowledge (PKC)](getting-started/external-pkc.md)

## User guide

- [Chat](user-guide/chat.md)
- [Vaults](user-guide/vaults.md)
- [Settings](user-guide/settings.md)
- [Providers](user-guide/providers.md)
- [Knowledge corpus](user-guide/knowledge-corpus.md)
- [Document ingestion](user-guide/document-ingestion.md)
- [Memory review](user-guide/memory-review.md)
- [Privacy and audit](user-guide/privacy-controls.md)
- [Backup and recovery](user-guide/backup-recovery.md)

## Concepts

- [Personal Knowledge Corpus](concepts/personal-knowledge-corpus.md)
- [Replaceable brain](concepts/replaceable-brain.md)
- [Provenance](concepts/provenance.md)
- [Memory, facts, and inference](concepts/memory-vs-inference.md)
- [Local-first operation](concepts/local-first.md)

## Architecture

- [Overview](architecture/overview.md)
- [Data flow](architecture/data-flow.md)
- [Vault architecture](architecture/vault-architecture.md)
- [Provider architecture](architecture/provider-architecture.md)
- [Corpus and vector architecture](architecture/corpus-architecture.md)
- [Ingestion pipeline](architecture/ingestion-pipeline.md)
- [Security boundaries](architecture/security-boundaries.md)

## Development and maintenance

- [Setup](development/setup.md)
- [Repository layout](development/repository-layout.md)
- [Build](development/build.md)
- [Testing](development/testing.md)
- [Tauri commands](development/tauri-commands.md)
- [Adding a provider](development/adding-a-provider.md)
- [Release process](development/release-process.md)
- [Maintainer guide](development/MAINTAINER-GUIDE.md)

## Operations and reference

- [Backup and restore operations](operations/backup-and-restore.md)
- [Logs and diagnostics](operations/logs-and-diagnostics.md)
- [Disaster recovery](operations/disaster-recovery.md)
- [Configuration reference](reference/configuration.md)
- [Storage reference](reference/storage.md)
- [Glossary](reference/glossary.md)
- [FAQ](FAQ.md)
- [Troubleshooting](TROUBLESHOOTING.md)

## Canonical project records

The root [STATE.md](../STATE.md) records verified capability, [ROADMAP.md](../ROADMAP.md)
records phase status, [REQUIREMENTS.md](../REQUIREMENTS.md) defines requirements,
and [DECISIONS.md](../DECISIONS.md) records architectural decisions. Older deep
design documents in this directory remain useful technical references and are
linked from the focused pages above.

## Deep technical references

- [Backup package design](BACKUP_AND_RECOVERY.md)
- [Corpus contract](CORPUS_CONTRACT.md)
- [Key management](KEY_MANAGEMENT.md)
- [Memory model](MEMORY_MODEL.md)
- [Permission model](PERMISSION_MODEL.md)
- [Provider design](PROVIDER_ARCHITECTURE.md)
- [Release acceptance specification](RELEASE_ACCEPTANCE.md)
- [Retrieval design](RETRIEVAL_ARCHITECTURE.md)
- [Vault model](VAULT_MODEL.md)
