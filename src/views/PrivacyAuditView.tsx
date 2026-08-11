import { useCallback, useEffect, useState } from "react";
import { api, explainError, type AuditEventView } from "@/lib/tauri";

export function PrivacyAuditView() {
  const [events, setEvents] = useState<AuditEventView[]>([]);
  const [counts, setCounts] = useState<[string, number][]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [evs, cnts] = await Promise.all([api.auditList(100), api.auditCounts()]);
      setEvents(evs);
      setCounts(cnts);
      setError(null);
    } catch (e) {
      setError(explainError(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div className="privacy-view">
      <section className="card">
        <h2 className="card-title">Audit summary</h2>
        {counts.length === 0 ? (
          <p className="muted">No audit events recorded yet.</p>
        ) : (
          <div className="row">
            {counts.map(([cat, n]) => (
              <span key={cat} className="badge">
                {cat}: {n}
              </span>
            ))}
          </div>
        )}
        <p className="muted small">
          Every cloud transmission, provider call, vault access, permission use, import,
          export, and backup is recorded here. Audit history is append-only.
        </p>
      </section>

      <section className="card">
        <h2 className="card-title">Recent events ({events.length})</h2>
        {events.length === 0 ? (
          <p className="muted">
            No events yet. Unlock a vault and send a chat message to see activity.
          </p>
        ) : (
          <table className="audit-table">
            <thead>
              <tr>
                <th>When</th>
                <th>Category</th>
                <th>Action</th>
                <th>Detail</th>
              </tr>
            </thead>
            <tbody>
              {events.map((e) => (
                <tr key={e.event_id}>
                  <td className="muted small">{e.occurred_at.slice(0, 19)}</td>
                  <td>
                    <span className="badge">{e.category}</span>
                  </td>
                  <td>{e.action}</td>
                  <td className="muted small">
                    {Object.keys(e.detail_json as object).length > 0
                      ? JSON.stringify(e.detail_json)
                      : ""}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {error && (
        <div className="card card-error">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}
