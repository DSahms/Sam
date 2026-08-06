import type { ReactNode } from "react";

// Shared placeholder for views whose implementing phase has not shipped yet.
// The directive forbids marking planned work complete or hiding that something
// is unfinished, so each placeholder says exactly what is and isn't built.
export function Placeholder({
  phase,
  summary,
  children,
}: {
  phase: string;
  summary: string;
  children?: ReactNode;
}) {
  return (
    <div className="placeholder">
      <p className="placeholder-summary">{summary}</p>
      <p className="placeholder-phase">Implements: {phase}</p>
      {children}
    </div>
  );
}
