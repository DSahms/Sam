# Calli Bench

Minimal Windows desktop test bench that runs the **real** Calli Archiviste
interview engine against a local KoboldCpp server. This is a development
tool for Stage 5 ("connect Sam") — it is NOT the product UI.

Two tabs:

- **Interview bench** — the Stage 3/4 free-form chapter interview and
  memoir writing (unchanged).
- **Probe chain (5-b)** — the Stage 5-b Mother Leeds walk: pick a mode and
  topic, walk surface -> sensory -> source -> contrast -> meaning, watch
  the level pips fill, skip out loud when needed, and land the harvest as
  candidates in the intake registry. Wording is the local model's job
  (persona-locked, static bank as the floor); approval is nobody's job
  here — the gate UI is Stage 5-b-4.

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
