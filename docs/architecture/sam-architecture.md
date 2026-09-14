# SAM Architecture

## Product Identity

- **Product Name:** SAM
- **Repository:** `C:\Users\david\ZCodeProject\Sammy` (directory name "Sammy" is temporary)
- **Default Assistant Name:** "Sam"
- **Legacy Product Name:** "Sammy" (must not be introduced into new code, documentation, UI, identifiers, or architecture)

## Component Architecture

### 1. Sam — Personal Assistant Application
- **Location:** `apps/sam/`
- **Description:** The primary personal assistant application. Provides conversational interface, memory management, and knowledge retrieval.
- **Default Identity:** Assistant name defaults to "Sam" but must be configurable by the user.
- **Identity Rule:** The assistant's displayed name/persona must eventually be configurable. Do not hard-code the assistant's identity architecture around the name "Sam".

### 2. Calli Archiviste — Personal Knowledge Intake Engine
- **Location:** `calli-archiviste/`
- **Technical Name:** "Personal Knowledge Intake Engine"
- **Human-Facing Identity:** "Calli" (the librarian persona)
- **Function:** Gathers information about artifacts through conversation/interview and submits structured knowledge to PKC.

### 3. PKC — Personal Knowledge Corpus
- **Location:** `pkc/`
- **Description:** The authoritative personal knowledge system. Durable knowledge storage and retrieval.
- **Source Repository:** `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus`
- **Status:** Currently external; to be migrated into this repository.

### 4. Media Archive
- **Location:** `media-archive/`
- **Description:** Preserves original artifacts (documents, audio, images, etc.).
- **Relationship to PKC:** The original artifact and the knowledge derived from it are separate but linked. Calli gathers information about artifacts through conversation/interview and submits structured knowledge to PKC.

### 5. Shared Infrastructure
- **Location:** `shared/`
- **Description:** Cross-cutting utilities, types, and infrastructure that genuinely belongs to Sam.
- **Scope:** Only components that are not specific to any single application within the SAM platform.

## Migration Rules

### From StoryKeeper-Local-Writer
- **Source:** `D:\dev\StoryKeeper-Local-Writer`
- **Status:** Donor/source repository only.
- **Future Role:** Will eventually become Calli Archiviste.
- **Current Status:** Do not rename, delete, reset, clean, commit, or otherwise modify.

### From StoryKeeper
- **Source:** `D:\dev\StoryKeeper`
- **Status:** NOT part of this migration. Ledger Series remains a separate product/repository.

### From PKC
- **Source:** `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus`
- **Status:** Authoritative PKC system. To be migrated into this repository.
- **Current Status:** Do not modify.

## Dependency Rules

- Sam components must not depend on unrelated external copies of Sam functionality.
- Do not create duplicate copies of the interview engine. Determine the authoritative implementation during migration.
- Calli Archiviste will eventually absorb the interview engine from StoryKeeper-Local-Writer.

## Naming Conventions

- **New Code:** Use "SAM" as the product identity.
- **Assistant Name:** Default to "Sam" but must be user-configurable.
- **Legacy References:** "Sammy" must not be introduced into new code, documentation, UI, identifiers, or architecture.
- **Existing Code:** Legacy "Sammy" references in existing code will require deliberate migration but should not be changed in this task.

## Repository Structure

```
Sam/
├── apps/
│   └── sam/
├── calli-archiviste/
├── pkc/
├── media-archive/
├── shared/
├── docs/
│   ├── architecture/
│   ├── migration/
│   └── decisions/
└── README.md
```