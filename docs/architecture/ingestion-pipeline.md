# Ingestion pipeline

[Document guide](../user-guide/document-ingestion.md) · [Security boundaries](security-boundaries.md)

```mermaid
flowchart LR
    F["Owner-selected file"] --> K["Extension classification"]
    K --> E["Format-specific local extractor"]
    E --> V{"Extraction valid?"}
    V -->|"no"| X["Sanitized failure; nothing indexed"]
    V -->|"yes"| H["SHA-256 duplicate check"]
    H -->|"existing"| ID["Return stable source ID"]
    H -->|"new"| ENC["AES-GCM encrypt original bytes"]
    ENC --> DB["Store metadata + extracted text"]
    DB --> FTS["Update active-source FTS5"]
```

PDF and DOCX parsers operate locally. DOCX extraction reads only the document XML.
Image OCR streams bytes into a local Tesseract process. Untrusted content becomes
data for retrieval; it is not treated as an instruction to Sammy’s application
runtime and does not execute embedded code.
