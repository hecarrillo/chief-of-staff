import { A, useLocation } from "@solidjs/router";

const NAV_ITEMS = [
  { path: "/", label: "Messages", key: "1" },
  { path: "/dashboard", label: "Dashboard", key: "2" },
  { path: "/sessions", label: "Sessions", key: "3" },
  { path: "/settings", label: "Settings", key: "4" },
];

export default function Sidebar() {
  const location = useLocation();

  return (
    <nav class="w-48 bg-surface border-r border-neutral-800 flex flex-col py-4">
      <div class="px-4 mb-6">
        <h1 class="text-sm font-bold text-terminal-green tracking-wide">CoS</h1>
        <p class="text-[10px] text-terminal-dim mt-0.5">Desktop</p>
      </div>

      <div class="flex flex-col gap-1 px-2">
        {NAV_ITEMS.map((item) => (
          <A
            href={item.path}
            class={`flex items-center justify-between px-3 py-2 rounded text-xs transition-colors ${
              location.pathname === item.path
                ? "bg-neutral-800 text-neutral-100"
                : "text-terminal-dim hover:text-neutral-300 hover:bg-surface-hover"
            }`}
          >
            <span>{item.label}</span>
            <kbd class="text-[9px] text-neutral-600">{item.key}</kbd>
          </A>
        ))}
      </div>
    </nav>
  );
}
