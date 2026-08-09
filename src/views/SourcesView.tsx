import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, explainError, type SourceView } from "@/lib/tauri";

export function SourcesView() {
  const [sources, setSources] = useState<SourceView[]>([]);
  const [search, setSearch] = useState("");
  const [searchHits, setSearchHits] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ id: string; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const list = await api.sourceList();
      setSources(list);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleImport = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        title: "Select a source file to import",
      });
      if (!selected) return;
      await api.sourceImport(selected as string);
      await refresh();
    } catch (e) {
      setError(explainError(e));
    } finally {
      setBusy(false);
    }
  }, [refresh]);

  const handleSearch = useCallback(async () => {
    if (!search.trim()) {
      setSearchHits(null);
      return;
    }
    try {
      const hits = await api.sourceSearch(search.trim());
      setSearchHits(hits);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, [search]);

  const handlePreview = useCallback(async (id: string) => {
    try {
      const text = await api.sourceExtracted(id);
      setPreview({ id, text: text.slice(0, 2000) });
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  const handleDelete = useCallback(
    async (id: string) => {
      try {
        await api.sourceDelete(id);
        await refresh();
      } catch (e) {
        setError(explainError(e));
      }
    },
    [refresh],
  );

  const filtered = sources.filter((s) => {
    if (searchHits) return searchHits.includes(s.source_id);
    return true;
  });

  return (
    <div className="sources-view">
      <section className="card">
        <h2 className="card-title">Import a source</h2>
        <p className="muted small">
          Files you select are encrypted at rest with AES-256-GCM. No macros or active
          content are executed. Only files you explicitly pick are read.
        </p>
        <button
          type="button"
          className="btn btn-primary"
          onClick={handleImport}
          disabled={busy}
        >
          {busy ? "Importing…" : "Choose file to import"}
        </button>
      </section>

      <section className="card">
        <h2 className="card-title">Search</h2>
        <div className="row">
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Lexical search over extracted text…"
            style={{ flex: 1 }}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
          <button type="button" className="btn" onClick={handleSearch}>
            Search
          </button>
          {searchHits && (
            <button
              type="button"
              className="btn"
              onClick={() => {
                setSearchHits(null);
                setSearch("");
              }}
            >
              Clear
            </button>
          )}
        </div>
      </section>

      <section className="card">
        <h2 className="card-title">Sources ({filtered.length})</h2>
        {filtered.length === 0 && <p className="muted">No sources. Import one above.</p>}
        <ul className="record-list">
          {filtered.map((s) => (
            <li key={s.source_id} className="record-item">
              <div className="record-header">
                <strong>{s.name}</strong>
                <span className="muted small">{s.kind}</span>
                <span className="muted small">{formatBytes(s.bytes_len)}</span>
                {s.status !== "active" && (
                  <span className="badge badge-warn">{s.status}</span>
                )}
              </div>
              <div className="muted small">checksum: {s.checksum.slice(0, 16)}…</div>
              <div className="record-actions">
                <button
                  type="button"
                  className="btn"
                  onClick={() => handlePreview(s.source_id)}
                >
                  Preview text
                </button>
                <button
                  type="button"
                  className="btn btn-danger"
                  onClick={() => handleDelete(s.source_id)}
                >
                  Delete
                </button>
              </div>
              {preview?.id === s.source_id && (
                <pre className="source-preview">{preview.text}</pre>
              )}
            </li>
          ))}
        </ul>
      </section>

      {error && (
        <div className="card card-error">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
