# Document ingestion

[Ingestion architecture](../architecture/ingestion-pipeline.md) · [Knowledge corpus](knowledge-corpus.md) · [Troubleshooting](../TROUBLESHOOTING.md)

**Sources** imports owner-selected files, encrypts original bytes with the vault
key, extracts searchable text, and records checksum, file kind, timestamps,
locations, and derivation metadata.

| Type | Extensions | Extraction |
| --- | --- | --- |
| Text/Markdown | `.txt`, `.md`, `.markdown` | UTF-8 text |
| Structured text | `.json`, `.csv` | Validated/normalized local parsing |
| Documents | `.pdf`, `.docx` | Local PDF text extraction; DOCX `word/document.xml` only |
| Images | `.png`, `.jpg`, `.jpeg`, `.webp` | Local Tesseract stdin/stdout OCR |

No document macros or embedded programs execute. Image bytes are not placed in a
plaintext temporary file. Unsupported formats and extraction failures are
reported rather than indexed as fabricated or lossy binary text. Scanned PDFs
without an extractable text layer are not automatically rasterized for OCR.

Reimporting active content with the same SHA-256 checksum returns the existing
source ID and does not duplicate FTS rows. Deleting a source marks it deleted and
removes it from active search.
