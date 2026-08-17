import { useCallback, useEffect, useState } from "react";
import { api, explainError, pkcClassLabel, type MemoryCandidateView } from "@/lib/tauri";

const STATE_FILTERS = [
  "pending",
  "approved",
  "rejected",
  "deferred",
  "temporary",
] as const;

export function MemoryReviewView() {
  const [candidates, setCandidates] = useState<MemoryCandidateView[]>([]);
  const [filter, setFilter] = useState<string>("pending");
  const [editing, setEditing] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const list = await api.memoryList(filter);
      setCandidates(list);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, [filter]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleApprove = useCallback(
    async (id: string, edited?: string) => {
      try {
        await api.memoryApprove(id, edited);
        setEditing(null);
        await refresh();
      } catch (e) {
        setError(explainError(e));
      }
    },
    [refresh],
  );

  return (
    <div className="memory-view">
      <section className="card">
        <h2 className="card-title">Memory candidates</h2>
        <p className="muted small">
          Conversation content and personal-knowledge lookups never become lasting
          memory automatically. Review each candidate: approve (promotes to a
          knowledge record), edit and approve, reject, defer, or mark temporary.
          Editing a candidate does not change the original personal-knowledge source.
        </p>
        <div className="row">
          {STATE_FILTERS.map((s) => (
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
        <h2 className="card-title">Candidates ({candidates.length})</h2>
        {candidates.length === 0 && <p className="muted">No candidates in this state.</p>}
        <ul className="record-list">
          {candidates.map((c) => (
            <li key={c.candidate_id} className="record-item">
              <div className="record-header">
                <span className={"badge badge-" + statusBadge(c.state)}>{c.state}</span>
                <span className="muted small">{c.record_type}</span>
                {c.origin === "external_pkc" && (
                  <span className="badge" title="Queued from a personal-knowledge chat turn">
                    from personal knowledge
                  </span>
                )}
                {c.epistemic_class && (
                  <span className="muted small">{pkcClassLabel(c.epistemic_class)}</span>
                )}
                {c.crossed_to_cloud && (
                  <span
                    className="badge badge-warn"
                    title="Source content crossed to a cloud provider"
                  >
                    crossed to cloud
                  </span>
                )}
                <span className="muted small">provider: {c.provider_used}</span>
              </div>
              {editing === c.candidate_id ? (
                <div className="row">
                  <input
                    type="text"
                    value={editText}
                    onChange={(e) => setEditText(e.target.value)}
                    style={{ flex: 1 }}
                  />
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => handleApprove(c.candidate_id, editText)}
                  >
                    Save & approve
                  </button>
                  <button type="button" className="btn" onClick={() => setEditing(null)}>
                    Cancel
                  </button>
                </div>
              ) : (
                <>
                  <div className="record-text">{c.proposed_text}</div>
                  {c.origin === "external_pkc" && (
                    <ul className="muted small pkc-provenance-meta">
                      {c.epistemic_class === "inference" && (
                        <li>This is a suggestion to review, not stored fact.</li>
                      )}
                      {c.epistemic_class === "testimony" && (
                        <li>This came from something you previously described.</li>
                      )}
                      {c.pkc_source_id && <li>Source: {c.pkc_source_id}</li>}
                      {c.owner_edited && c.original_proposed_text && (
                        <li>Original proposal: {c.original_proposed_text}</li>
                      )}
                      {c.conflict_record_id && (
                        <li>
                          Conflicts with an existing lasting memory (
                          {c.conflict_record_id}). Both are kept until you decide.
                        </li>
                      )}
                    </ul>
                  )}
                  {c.reason && <div className="muted small">reason: {c.reason}</div>}
                  <div className="record-actions">
                    {c.state === "pending" && (
                      <>
                        <button
                          type="button"
                          className="btn btn-primary"
                          onClick={() => handleApprove(c.candidate_id)}
                        >
                          Approve
                        </button>
                        <button
                          type="button"
                          className="btn"
                          onClick={() => {
                            setEditing(c.candidate_id);
                            setEditText(c.proposed_text);
                          }}
                        >
                          Edit & approve
                        </button>
                        <button
                          type="button"
                          className="btn"
                          onClick={() =>
                            void api.memoryReject(c.candidate_id).then(refresh)
                          }
                        >
                          Reject
                        </button>
                        <button
                          type="button"
                          className="btn"
                          onClick={() =>
                            void api.memoryDefer(c.candidate_id).then(refresh)
                          }
                        >
                          Defer
                        </button>
                        <button
                          type="button"
                          className="btn"
                          onClick={() =>
                            void api.memoryMarkTemporary(c.candidate_id).then(refresh)
                          }
                        >
                          Temporary
                        </button>
                      </>
                    )}
                    <button
                      type="button"
                      className="btn btn-danger"
                      onClick={() => void api.memoryDelete(c.candidate_id).then(refresh)}
                    >
                      Delete
                    </button>
                  </div>
                </>
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

function statusBadge(state: string): string {
  switch (state) {
    case "approved":
      return "ok";
    case "pending":
      return "warn";
    case "rejected":
      return "error";
    default:
      return "";
  }
}
