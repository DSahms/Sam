# Glossary

[Documentation home](../README.md) · [FAQ](../FAQ.md)

- **Audit event:** append-only structured record of a security- or behavior-relevant action.
- **API:** a defined interface that lets software exchange requests and responses.
- **API endpoint:** the network address where a software service accepts requests.
- **Argon2id:** an algorithm designed to make large-scale passphrase guessing expensive.
- **Backup:** encrypted, versioned package of a vault for owner-controlled storage.
- **Context:** selected identity and retrieved information supplied to a model for one turn.
- **Corpus:** canonical owner-controlled identity, knowledge, sources, history, and reviewed memory.
- **Cloud provider:** model service reached over the Internet; currently Venice.
- **Inference:** model conclusion that is not automatically stored as fact.
- **Ingestion:** classification, local extraction, encryption, metadata storage, and indexing of a source.
- **Embedding:** numeric representation derived from text for similarity comparison; it is not canonical knowledge.
- **Encryption key:** secret random data used to protect or unlock information.
- **Environment variable:** a named setting supplied by the operating environment, such as `PATH`.
- **FTS5:** SQLite's full-text-search index used for word matching.
- **Local provider:** model service intended to run on the device/localhost; currently KoboldCpp-compatible.
- **Localhost:** a network name meaning “this same computer.”
- **Memory candidate:** proposed durable knowledge awaiting owner review.
- **Model:** the language model selected behind a provider.
- **Personal Knowledge Corpus:** Sammy’s persistent model-independent knowledge layer.
- **Provider:** adapter that lists models, reports health, and performs chat completion.
- **Provenance:** record of where information originated and how it was derived.
- **Recovery code:** one-time-displayed high-entropy credential that independently unwraps a vault key.
- **Retrieval:** selection and ranking of permitted current corpus records for context.
- **Source:** imported encrypted file plus extracted text and provenance metadata.
- **SQLCipher:** SQLite-compatible database encryption protecting each vault at rest.
- **Tesseract/OCR:** local software that turns text visible in an image into searchable characters.
- **Database migration:** a versioned change updating an older vault's table structure.
- **Package manager:** a dependency tool; npm manages frontend packages and Cargo manages Rust crates.
- **Port:** a numbered connection point for a service; KoboldCpp commonly uses `5001`.
- **Vault:** independent encrypted storage, key, corpus, settings, indexes, and audit boundary.
- **Vector index:** derived similarity index; rebuildable and never authoritative.
