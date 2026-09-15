# Calli Bench

Minimal Windows desktop test bench that runs the **real** Calli Archiviste
interview engine against a local KoboldCpp server. This is a development
tool for Stage 5 ("connect Sam") — it is NOT the product UI.

## What it proves

- The extracted engine (interviewer prompts, follow_up prompts) drives a
  live chapter interview.
- Stage 3 listening works: when an answer carries emotional weight, the
  next question stays on that thread (probe-deeper traces print to the
  run console).
- Stage 3 compression works: long sessions compress older messages into
  an "Earlier in this conversation" tier instead of dropping them.
- Stage 4 memoir works: completed sessions become first-person prose via
  the narrative prompts, saved with a content fingerprint.
- The M40 KoboldCpp endpoint (127.0.0.1:5001, enforced loopback) answers.

## Prerequisites

- Flutter (Windows desktop enabled) and Visual Studio C++ toolchain.
- KoboldCpp serving on this machine at `http://127.0.0.1:5001`.

## Run (first time)

From `apps/calli-bench`:

    flutter create . --platforms=windows
    flutter pub get
    flutter run -d windows

## Where the data lives

Interview sessions and memoirs are stored in Hive boxes under
`Documents\calli_bench_data\`. Personal content never enters Git
(migration anti-pattern 3).
