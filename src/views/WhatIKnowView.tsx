import { useCallback, useEffect, useState } from "react";
import { api, explainError, type KnowledgeRecordView } from "@/lib/tauri";

const STATUS_FILTERS = [
  "all",
  "approved",
  "candidate",
  "disputed",
  "rejected",
  "superseded",
  "tombstoned",
] as const;
type StatusFilter = (typeof STATUS_FILTERS)[number];

export function WhatIKnowView() {
  const [records, setRecords] = useState<KnowledgeRecordView[]>([]);
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [search, setSearch] = useState("");
  const [searchHits, setSearchHits] = useState<string[] | null>(null);
  const [newFact, setNewFact] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [correcting, setCorrecting] = useState<string | null>(null);
  const [correctText, setCorrectText] = useState("");

  const refresh = useCallback(async () => {
    try {
      const list = await api.knowledgeList();
      setRecords(list);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleAdd = useCallback(async () => {
    if (!newFact.trim()) return;
    try {
      await api.knowledgeAddFact(newFact.trim());
      setNewFact("");
      await refresh();
    } catch (e) {
      setError(explainError(e));
    }
  }, [newFact, refresh]);

  const handleSearch = useCallback(async () => {
    if (!search.trim()) {
      setSearchHits(null);
      return;
    }
    try {
      const hits = await api.knowledgeSearch(search.trim());
      setSearchHits(hits);
    } catch (e) {
      setError(explainError(e));
    }
  }, [search]);

  const handleCorrect = useCallback(
    async (id: string) => {
      if (!correctText.trim()) return;
      try {
        await api.knowledgeCorrect(id, correctText.trim());
        setCorrecting(null);
        setCorrectText("");
        await refresh();
      } catch (e) {
        setError(explainError(e));
      }
    },
    [correctText, refresh],
  );

  const filtered = records.filter((r) => {
    if (filter !== "all" && r.status !== filter) return false;
    if (searchHits) {
      return searchHits.includes(r.record_id);
    }
    return true;
  });

  return (
    <div className="whatiknow-view">
      <section className="card">
        <h2 className="card-title">Add a fact</h2>
        <div className="row">
          <input
            type="text"
            value={newFact}
            onChange={(e) => setNewFact(e.target.value)}
            placeholder="Enter a fact the owner approves directly…"
            style={{ flex: 1 }}
            onKeyDown={(e) => e.key === "Enter" && handleAdd()}
          />
          <button type="button" className="btn btn-primary" onClick={handleAdd}>
            Add approved fact
          </button>
        </div>
        <p className="muted small">
          Manually entered facts are approved immediately. AI-proposed records always
          start as candidates awaiting review.
        </p>
      </section>

      <section className="card">
        <h2 className="card-title">Search</h2>
        <div className="row">
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Lexical search over current knowledge…"
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
        <div className="row">
          {STATUS_FILTERS.map((s) => (
            <button
              key={s}
              type="button"
              className={"btn" + (filter === s ? " btn-primary" : "")}
              onClick={() => setFilter(s)}
            >
              {s}
            </button>
          ))}
        </div>
      </section>

      <section className="card">
        <h2 className="card-title">Records ({filtered.length})</h2>
        {filtered.length === 0 && (
          <p className="muted">No records match the current filter.</p>
        )}
        <ul className="record-list">
          {filtered.map((r) => (
            <li key={r.record_id} className="record-item">
              <div className="record-header">
                <span className={"badge badge-" + statusBadge(r.status)}>{r.status}</span>
                <span className="muted small">{r.record_type}</span>
                {r.sensitivity !== "normal" && (
                  <span className="badge badge-warn">{r.sensitivity}</span>
                )}
                {r.superseded_by && <span className="muted small">→ superseded</span>}
                {r.supersedes && <span className="muted small">← corrects</span>}
                {r.contradiction_set && (
                  <span className="badge badge-warn">contradiction</span>
                )}
              </div>
              <div className="record-text">{r.canonical_text}</div>
              {correcting === r.record_id ? (
                <div className="row">
                  <input
                    type="text"
                    value={correctText}
                    onChange={(e) => setCorrectText(e.target.value)}
                    placeholder="Corrected text…"
                    style={{ flex: 1 }}
                  />
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => handleCorrect(r.record_id)}
                  >
                    Save correction
                  </button>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => setCorrecting(null)}
                  >
                    Cancel
                  </button>
                </div>
              ) : (
                <div className="record-actions">
                  {r.status === "candidate" && (
                    <>
                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={() =>
                          void api.knowledgeApprove(r.record_id).then(refresh)
                        }
                      >
                        Approve
                      </button>
                      <button
                        type="button"
                        className="btn"
                        onClick={() =>
                          void api.knowledgeReject(r.record_id).then(refresh)
                        }
                      >
                        Reject
                      </button>
                    </>
                  )}
                  {(r.status === "approved" || r.status === "disputed") && (
                    <>
                      <button
                        type="button"
                        className="btn"
                        onClick={() => {
                          setCorrecting(r.record_id);
                          setCorrectText(r.canonical_text);
                        }}
                      >
                        Correct
                      </button>
                      <button
                        type="button"
                        className="btn btn-danger"
                        onClick={() =>
                          void api.knowledgeTombstone(r.record_id).then(refresh)
                        }
                      >
                        Tombstone
                      </button>
                    </>
                  )}
                </div>
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

function statusBadge(status: string): string {
  switch (status) {
    case "approved":
      return "ok";
    case "candidate":
      return "warn";
    case "rejected":
    case "tombstoned":
      return "error";
    default:
      return "";
  }
}
