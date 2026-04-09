import { Router, Route, useLocation } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import Sidebar from "./components/Sidebar";
import StatusBar from "./components/StatusBar";
import TodoPanel from "./components/TodoPanel";
import Messages from "./pages/Messages";
import Dashboard from "./pages/Dashboard";
import Sessions from "./pages/Sessions";
import Settings from "./pages/Settings";
import Setup from "./pages/Setup";

function Layout(props: { children?: any }) {
  const location = useLocation();
  const showTodo = () => location.pathname === "/" || location.pathname === "";

  return (
    <div class="flex flex-col h-screen bg-neutral-950">
      <div class="flex flex-1 overflow-hidden">
        <Sidebar />
        <main class="flex-1 overflow-hidden">{props.children}</main>
        <Show when={showTodo()}>
          <div class="w-56">
            <TodoPanel />
          </div>
        </Show>
      </div>
      <StatusBar />
    </div>
  );
}

export default function App() {
  const [setupDone, setSetupDone] = createSignal(false);
  const [checking, setChecking] = createSignal(true);

  // Quick check if config exists
  import("@tauri-apps/api/core").then(({ invoke }) => {
    invoke("check_system").then((sys: any) => {
      if (sys.config_exists) {
        setSetupDone(true);
      }
      setChecking(false);
    }).catch(() => setChecking(false));
  });

  return (
    <Show when={!checking()} fallback={<div class="h-screen bg-neutral-950" />}>
      <Show
        when={setupDone()}
        fallback={<Setup onComplete={() => setSetupDone(true)} />}
      >
        <Router root={Layout}>
          <Route path="/" component={Messages} />
          <Route path="/dashboard" component={Dashboard} />
          <Route path="/sessions" component={Sessions} />
          <Route path="/settings" component={Settings} />
        </Router>
      </Show>
    </Show>
  );
}
