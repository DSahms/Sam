//! `sources` — secure source ingestion (Phase 4).
//!
//! Directive §22/§27. Sources are files the owner explicitly selects through a
//! file picker (no arbitrary filesystem scanning). Originals are encrypted at
//! rest with AES-256-GCM; extracted text is stored alongside. No macro, script,
//! or active content is ever executed. No plaintext temporary files are left
//! behind after processing.
//!
//! This slice implements encrypted source storage + txt/markdown/json/csv
//! extractors + FTS indexing of extracted text. PDF/DOCX/image/OCR adapters are
//! replaceable behind the [`Extractor`] trait; PDF/DOCX/OCR are recorded as
//! external blockers until their bindings are wired (§6/§23).

use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::crypto::{aead, SecretKey};
use crate::error::{AppError, AppResult};
use crate::ids::SourceId;

/// The kind of a source, after extraction classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Text,
    Markdown,
    Json,
    Csv,
    Pdf,
    Docx,
    Image,
    Unknown,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Text => "text",
            SourceKind::Markdown => "markdown",
            SourceKind::Json => "json",
            SourceKind::Csv => "csv",
            SourceKind::Pdf => "pdf",
            SourceKind::Docx => "docx",
            SourceKind::Image => "image",
            SourceKind::Unknown => "unknown",
        }
    }

    /// Classify from a filename extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "txt" => SourceKind::Text,
            "md" | "markdown" => SourceKind::Markdown,
            "json" => SourceKind::Json,
            "csv" => SourceKind::Csv,
            "pdf" => SourceKind::Pdf,
            "docx" => SourceKind::Docx,
            "png" | "jpg" | "jpeg" | "webp" => SourceKind::Image,
            _ => SourceKind::Unknown,
        }
    }
}

/// Replaceable extraction adapter. Each format has its own implementation; all
/// produce sanitized text and never execute active content.
pub trait Extractor: Send + Sync {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText>;
}

/// The output of extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    pub text: String,
    /// Detected structure hints (e.g. "page:1", "heading:Intro").
    pub locations: Vec<String>,
    /// "extracted" for OCR/PDF-derived; "original" for plain text.
    pub derivation: String,
}

// =============================================================================
// Built-in extractors
// =============================================================================

/// Plain-text extractor: validates UTF-8, no execution.
pub struct TextExtractor;
impl Extractor for TextExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let text = String::from_utf8_lossy(bytes).into_owned();
        Ok(ExtractedText {
            text,
            locations: vec![],
            derivation: "original".into(),
        })
    }
}

/// Markdown extractor: stores as-is (markdown is plain text); records line count.
pub struct MarkdownExtractor;
impl Extractor for MarkdownExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let lines = text.lines().count();
        Ok(ExtractedText {
            text,
            locations: vec![format!("lines:1-{lines}")],
            derivation: "original".into(),
        })
    }
}

/// JSON extractor: pretty-prints valid JSON; records the top-level type.
pub struct JsonExtractor;
impl Extractor for JsonExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let v: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|e| AppError::InvalidArgument(format!("invalid JSON: {e}")))?;
        let text = serde_json::to_string_pretty(&v)?;
        let ty = match v {
            serde_json::Value::Object(_) => "object",
            serde_json::Value::Array(_) => "array",
            _ => "scalar",
        };
        Ok(ExtractedText {
            text,
            locations: vec![format!("json_root:{ty}")],
            derivation: "original".into(),
        })
    }
}

/// CSV extractor: joins rows into text; records row/column count.
pub struct CsvExtractor;
impl Extractor for CsvExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let raw = String::from_utf8_lossy(bytes).into_owned();
        let mut rows = 0usize;
        let mut cols = 0usize;
        for line in crate::sources::csv_lines(&raw) {
            rows += 1;
            cols = cols.max(line.len());
        }
        Ok(ExtractedText {
            text: raw,
            locations: vec![format!("rows:{rows},cols:{cols}")],
            derivation: "original".into(),
        })
    }
}

/// PDF extractor backed by `pdf-extract`. The document is parsed entirely in
/// memory; no plaintext temporary file is created.
pub struct PdfExtractor;
impl Extractor for PdfExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let text = pdf_extract::extract_text_from_mem(bytes).map_err(|_| {
            AppError::InvalidArgument("unable to extract text from PDF".into())
        })?;
        if text.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "PDF contains no extractable text; scanned pages require OCR".into(),
            ));
        }
        let pages = text.matches('\u{000c}').count().saturating_add(1);
        Ok(ExtractedText {
            text,
            locations: (1..=pages).map(|page| format!("page:{page}")).collect(),
            derivation: "extracted".into(),
        })
    }
}

/// DOCX extractor. DOCX is a ZIP container; only WordprocessingML text nodes
/// from `word/document.xml` are read. Macros and embedded objects are ignored.
pub struct DocxExtractor;
impl Extractor for DocxExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|_| AppError::InvalidArgument("invalid DOCX package".into()))?;
        let mut document = archive
            .by_name("word/document.xml")
            .map_err(|_| AppError::InvalidArgument("DOCX has no document body".into()))?;
        let mut xml = String::new();
        document.read_to_string(&mut xml).map_err(|_| {
            AppError::InvalidArgument("DOCX document XML is invalid".into())
        })?;

        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(false);
        let mut text = String::new();
        let mut paragraphs = 0usize;
        let mut in_text = false;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(event)) => {
                    match event.local_name().as_ref() {
                        b"t" => in_text = true,
                        b"tab" if in_text => text.push('\t'),
                        b"br" if in_text => text.push('\n'),
                        _ => {}
                    }
                }
                Ok(quick_xml::events::Event::Text(event)) if in_text => {
                    let decoded = event.unescape().map_err(|_| {
                        AppError::InvalidArgument("DOCX text is invalid".into())
                    })?;
                    text.push_str(&decoded);
                }
                Ok(quick_xml::events::Event::End(event)) => {
                    match event.local_name().as_ref() {
                        b"t" => in_text = false,
                        b"p" => {
                            paragraphs += 1;
                            text.push('\n');
                        }
                        _ => {}
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => {
                    return Err(AppError::InvalidArgument("DOCX XML is invalid".into()))
                }
                _ => {}
            }
        }
        if text.trim().is_empty() {
            return Err(AppError::InvalidArgument("DOCX contains no text".into()));
        }
        Ok(ExtractedText {
            text: text.trim_end().to_string(),
            locations: vec![format!("paragraphs:{paragraphs}")],
            derivation: "extracted".into(),
        })
    }
}

/// Local OCR adapter using Tesseract's stdin/stdout mode. Image bytes never
/// leave the machine and no plaintext output file is created. A missing OCR
/// runtime fails clearly instead of fabricating or indexing binary data.
pub struct TesseractOcrExtractor;
impl Extractor for TesseractOcrExtractor {
    fn extract(&self, bytes: &[u8]) -> AppResult<ExtractedText> {
        let mut child = Command::new("tesseract")
            .args(["stdin", "stdout", "--psm", "3"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| AppError::Config("local OCR runtime is not installed".into()))?;
        child
            .stdin
            .take()
            .ok_or_else(|| AppError::Config("local OCR input unavailable".into()))?
            .write_all(bytes)?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(AppError::InvalidArgument(
                "OCR could not read this image".into(),
            ));
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|_| AppError::InvalidArgument("OCR returned invalid text".into()))?;
        if text.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "OCR found no readable text".into(),
            ));
        }
        Ok(ExtractedText {
            text,
            locations: vec!["image:1".into()],
            derivation: "ocr".into(),
        })
    }
}

/// Minimal RFC-4180-ish CSV row splitter (no external dep).
pub(crate) fn csv_lines(input: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for line in input.lines() {
        let mut row = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        for ch in line.chars() {
            match ch {
                '"' if in_quotes => in_quotes = false,
                '"' => in_quotes = true,
                ',' if !in_quotes => {
                    row.push(std::mem::take(&mut cur));
                }
                _ => cur.push(ch),
            }
        }
        row.push(cur);
        out.push(row);
    }
    out
}

/// Pick the right extractor for a source kind.
pub fn extractor_for(kind: SourceKind) -> Box<dyn Extractor> {
    match kind {
        SourceKind::Text => Box::new(TextExtractor),
        SourceKind::Markdown => Box::new(MarkdownExtractor),
        SourceKind::Json => Box::new(JsonExtractor),
        SourceKind::Csv => Box::new(CsvExtractor),
        SourceKind::Pdf => Box::new(PdfExtractor),
        SourceKind::Docx => Box::new(DocxExtractor),
        SourceKind::Image => Box::new(TesseractOcrExtractor),
        SourceKind::Unknown => Box::new(UnsupportedExtractor),
    }
}

struct UnsupportedExtractor;
impl Extractor for UnsupportedExtractor {
    fn extract(&self, _bytes: &[u8]) -> AppResult<ExtractedText> {
        Err(AppError::InvalidArgument(
            "unsupported source format".into(),
        ))
    }
}

// =============================================================================
// Encrypted source storage
// =============================================================================

/// A stored source's metadata row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub source_id: String,
    pub kind: String,
    pub name: String,
    pub checksum: String,
    pub imported_at: String,
    pub status: String,
    pub bytes_len: u64,
}

/// Ensure the sources table has the Phase 4 columns (idempotent). The v4
/// migration creates the basic table; this adds the Phase 4 columns.
pub fn ensure_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sources (
            source_id    TEXT PRIMARY KEY,
            kind         TEXT NOT NULL,
            name         TEXT NOT NULL,
            checksum     TEXT NOT NULL,
            imported_at  TEXT NOT NULL,
            status       TEXT NOT NULL DEFAULT 'active',
            bytes_len    INTEGER NOT NULL DEFAULT 0,
            enc_blob     BLOB NOT NULL,
            extracted    TEXT NOT NULL DEFAULT ''
        );
         CREATE VIRTUAL TABLE IF NOT EXISTS sources_fts USING fts5(
            source_id UNINDEXED, name, extracted
         );",
    )?;
    // Forward-compatible additions for extraction provenance. Existing vaults
    // may already have the Phase 4 table, so duplicate-column errors are safe.
    let _ = conn.execute(
        "ALTER TABLE sources ADD COLUMN extraction_locations TEXT NOT NULL DEFAULT '[]'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sources ADD COLUMN extraction_derivation TEXT NOT NULL DEFAULT 'original'",
        [],
    );
    Ok(())
}

/// Import a file: encrypt the original bytes, extract text, store both, index
/// the extracted text. Returns the new source id. The DEK comes from the
/// unlocked vault session.
pub fn import(
    conn: &Connection,
    dek: &SecretKey,
    file_path: &Path,
) -> AppResult<SourceId> {
    ensure_schema(conn)?;
    let bytes = std::fs::read(file_path)?;
    let name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".into());
    let ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let kind = SourceKind::from_extension(&ext);

    // Extract text BEFORE encrypting, into memory only (no temp file on disk).
    let extracted = extractor_for(kind).extract(&bytes)?;
    let checksum = hex::encode(Sha256::digest(&bytes));

    // Idempotent re-import: identical active content resolves to the existing
    // source, preserving stable provenance and avoiding duplicate FTS rows.
    if let Ok(existing) = conn.query_row(
        "SELECT source_id FROM sources WHERE checksum=?1 AND status='active' LIMIT 1",
        params![checksum.as_str()],
        |row| row.get::<_, String>(0),
    ) {
        return SourceId::parse(&existing);
    }

    // Encrypt the original bytes with the vault DEK.
    let enc_blob = aead::encrypt(dek, &bytes);

    let id = SourceId::new();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO sources(source_id, kind, name, checksum, imported_at, status,
                             bytes_len, enc_blob, extracted, extraction_locations,
                             extraction_derivation)
         VALUES (?1,?2,?3,?4,?5,'active',?6,?7,?8,?9,?10)",
        params![
            id.to_string(),
            kind.as_str(),
            name,
            checksum,
            now,
            bytes.len() as i64,
            enc_blob,
            extracted.text,
            serde_json::to_string(&extracted.locations)?,
            extracted.derivation,
        ],
    )?;
    // Index name + extracted text in FTS.
    conn.execute(
        "INSERT INTO sources_fts(source_id, name, extracted) VALUES (?1,?2,?3)",
        params![id.to_string(), name, extracted.text],
    )?;
    Ok(id)
}

/// List all sources.
pub fn list(conn: &Connection) -> AppResult<Vec<SourceRecord>> {
    ensure_schema(conn)?;
    let mut stmt = conn.prepare(
        "SELECT source_id, kind, name, checksum, imported_at, status, bytes_len
         FROM sources ORDER BY imported_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SourceRecord {
            source_id: r.get::<_, String>(0)?,
            kind: r.get::<_, String>(1)?,
            name: r.get::<_, String>(2)?,
            checksum: r.get::<_, String>(3)?,
            imported_at: r.get::<_, String>(4)?,
            status: r.get::<_, String>(5)?,
            bytes_len: r.get::<_, i64>(6)? as u64,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Fetch the extracted text for a source.
pub fn extracted_text(conn: &Connection, id: SourceId) -> AppResult<String> {
    let text: String = conn
        .query_row(
            "SELECT extracted FROM sources WHERE source_id=?1",
            params![id.to_string()],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("source".into()),
            other => AppError::from(other),
        })?;
    Ok(text)
}

/// Decrypt and return the original bytes of a source (for export).
pub fn original_bytes(
    conn: &Connection,
    dek: &SecretKey,
    id: SourceId,
) -> AppResult<Vec<u8>> {
    let blob: Vec<u8> = conn
        .query_row(
            "SELECT enc_blob FROM sources WHERE source_id=?1",
            params![id.to_string()],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("source".into()),
            other => AppError::from(other),
        })?;
    let pt = aead::decrypt(dek, &blob).map_err(|_| AppError::Crypto)?;
    Ok(pt.as_slice().to_vec())
}

/// Delete a source: removes the source row (directive §19: deleted sources
/// stop appearing in retrieval). The [`search`] join filters on
/// `sources.status='active'`, so deleting the source row is sufficient to
/// remove it from results; we also best-effort remove the FTS row.
pub fn delete(conn: &Connection, id: SourceId) -> AppResult<()> {
    // Soft-delete: mark the source as 'deleted' so the active-status filter in
    // search excludes it. Hard-deleting FTS rows can destabilize some FTS5
    // builds; the status filter is authoritative for retrieval (directive §19).
    conn.execute(
        "UPDATE sources SET status='deleted' WHERE source_id=?1",
        params![id.to_string()],
    )?;
    Ok(())
}

/// Lexical search over source extracted text. Returns active source ids only.
///
/// The query is sanitized into an FTS5 phrase query (double-quoted) so that
/// special characters in the user's input (hyphens, asterisks, parentheses,
/// `AND`/`OR`/`NOT`) are treated as literal text rather than FTS5 query
/// operators. This avoids malformed-query errors and prevents operator-style
/// injection. Internal double quotes are escaped as `""` per FTS5 rules.
pub fn search(conn: &Connection, query: &str, limit: u32) -> AppResult<Vec<String>> {
    let sanitized = sanitize_fts_phrase(query);
    // Collect active ids first (drop that statement before running FTS), since
    // SQLite allows only one live reading statement per connection.
    let mut active_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    {
        let mut active =
            conn.prepare("SELECT source_id FROM sources WHERE status='active'")?;
        let act_rows = active.query_map([], |r| r.get::<_, String>(0))?;
        for r in act_rows {
            active_ids.insert(r?);
        }
    }
    let mut out = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT source_id FROM sources_fts WHERE sources_fts MATCH ?1")?;
        let fts_rows =
            stmt.query_map(params![sanitized.as_str()], |r| r.get::<_, String>(0))?;
        for r in fts_rows {
            let id = r?;
            if active_ids.contains(&id) {
                out.push(id);
                if out.len() >= limit as usize {
                    break;
                }
            }
        }
    }
    Ok(out)
}

/// Turn arbitrary user input into a safe FTS5 phrase query: wrap in double
/// quotes and escape any embedded double quotes by doubling them.
fn sanitize_fts_phrase(query: &str) -> String {
    let escaped = query.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        ensure_schema(&c).unwrap();
        c
    }

    #[test]
    fn text_extractor_round_trip() {
        let e = TextExtractor;
        let r = e.extract(b"hello world").unwrap();
        assert_eq!(r.text, "hello world");
        assert_eq!(r.derivation, "original");
    }

    #[test]
    fn markdown_extractor_counts_lines() {
        let e = MarkdownExtractor;
        let r = e.extract(b"# Title\nbody\nmore").unwrap();
        assert!(r.locations[0].contains("1-3"), "got: {}", r.locations[0]);
    }

    #[test]
    fn json_extractor_pretty_prints() {
        let e = JsonExtractor;
        let r = e.extract(br#"{"a":1}"#).unwrap();
        assert!(r.text.contains("\"a\": 1"));
        assert!(r.locations[0].contains("object"));
    }

    #[test]
    fn json_extractor_rejects_invalid() {
        let e = JsonExtractor;
        assert!(e.extract(b"not json").is_err());
    }

    #[test]
    fn csv_extractor_counts_rows_cols() {
        let e = CsvExtractor;
        let r = e.extract(b"a,b,c\n1,2,3").unwrap();
        assert!(r.locations[0].contains("rows:2"));
        assert!(r.locations[0].contains("cols:3"));
    }

    #[test]
    fn csv_lines_handles_quoted_commas() {
        let rows = csv_lines("\"a,b\",c\nd,e");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a,b".to_string(), "c".into()]);
    }

    #[test]
    fn docx_extractor_reads_document_text_without_embedded_content() {
        let mut bytes = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
            zip.start_file::<_, ()>(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(
                br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello &amp; safe</w:t></w:r></w:p><w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p></w:body></w:document>"#,
            )
            .unwrap();
            zip.finish().unwrap();
        }
        let result = DocxExtractor.extract(&bytes).unwrap();
        assert_eq!(result.text, "Hello & safe\nSecond paragraph");
        assert_eq!(result.locations, vec!["paragraphs:2"]);
        assert_eq!(result.derivation, "extracted");
    }

    #[test]
    fn invalid_docx_and_pdf_fail_instead_of_indexing_binary_data() {
        assert!(DocxExtractor.extract(b"not a zip").is_err());
        assert!(PdfExtractor.extract(b"not a pdf").is_err());
        assert!(UnsupportedExtractor.extract(b"binary").is_err());
    }

    #[test]
    fn import_encrypts_and_indexes_text() {
        let conn = fresh_conn();
        let dek = SecretKey::random();
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("note.txt");
        std::fs::write(&p, b"the eagle has landed").unwrap();
        let id = import(&conn, &dek, &p).unwrap();

        let extracted = extracted_text(&conn, id).unwrap();
        assert_eq!(extracted, "the eagle has landed");

        // Original bytes decrypt back.
        let orig = original_bytes(&conn, &dek, id).unwrap();
        assert_eq!(orig, b"the eagle has landed");

        // Searchable.
        let hits = search(&conn, "eagle", 10).unwrap();
        assert!(hits.contains(&id.to_string()));

        // The stored blob is not plaintext.
        let blob: Vec<u8> = conn
            .query_row(
                "SELECT enc_blob FROM sources WHERE source_id=?1",
                params![id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!blob.starts_with(b"the eagle"));
    }

    #[test]
    fn delete_removes_from_search() {
        let conn = fresh_conn();
        let dek = SecretKey::random();
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("n.txt");
        std::fs::write(&p, b"unique-token-here").unwrap();
        let id = import(&conn, &dek, &p).unwrap();
        assert!(!search(&conn, "unique-token", 10).unwrap().is_empty());
        delete(&conn, id).unwrap();
        assert!(search(&conn, "unique-token", 10).unwrap().is_empty());
    }

    #[test]
    fn kind_classification_from_extension() {
        assert_eq!(SourceKind::from_extension("txt"), SourceKind::Text);
        assert_eq!(SourceKind::from_extension("MD"), SourceKind::Markdown);
        assert_eq!(SourceKind::from_extension("pdf"), SourceKind::Pdf);
        assert_eq!(SourceKind::from_extension("png"), SourceKind::Image);
        assert_eq!(SourceKind::from_extension("weird"), SourceKind::Unknown);
    }

    #[test]
    fn import_records_checksum() {
        let conn = fresh_conn();
        let dek = SecretKey::random();
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, b"abc").unwrap();
        let id = import(&conn, &dek, &p).unwrap();
        let rec = list(&conn)
            .unwrap()
            .into_iter()
            .find(|r| r.source_id == id.to_string())
            .unwrap();
        let expected = hex::encode(Sha256::digest(b"abc"));
        assert_eq!(rec.checksum, expected);
    }

    #[test]
    fn repeated_import_is_idempotent_and_preserves_extraction_provenance() {
        let conn = fresh_conn();
        let dek = SecretKey::random();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, b"# Heading\nBody").unwrap();
        let first = import(&conn, &dek, &path).unwrap();
        let second = import(&conn, &dek, &path).unwrap();
        assert_eq!(first, second);
        let (locations, derivation): (String, String) = conn
            .query_row(
                "SELECT extraction_locations, extraction_derivation FROM sources WHERE source_id=?1",
                [first.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(locations.contains("lines:1-2"));
        assert_eq!(derivation, "original");
        assert_eq!(list(&conn).unwrap().len(), 1);
    }
}
