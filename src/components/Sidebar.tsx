import { navOrder, views, type ViewId } from "./navigation";

// Primary navigation. The eight top-level destinations come directly from
// directive §33. Each view placeholder states the phase that implements it.
export function Sidebar({
  active,
  onSelect,
}: {
  active: ViewId;
  onSelect: (v: ViewId) => void;
}) {
  return (
    <nav className="sidebar" aria-label="Primary">
      <div className="sidebar-brand">
        <span className="sidebar-mark" aria-hidden="true">
          S
        </span>
        <span className="sidebar-name">Sammy</span>
      </div>
      <ul>
        {navOrder.map((id) => (
          <li key={id}>
            <button
              type="button"
              className={"nav-item" + (active === id ? " active" : "")}
              onClick={() => onSelect(id)}
              aria-current={active === id ? "page" : undefined}
            >
              {views[id].label}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
