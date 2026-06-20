import { type CSSProperties, StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { HdrTuning } from "./HdrTuning";
import { Statistics } from "./Statistics";

type Tab = "logs" | "stats" | "hdr";

function tabButton(active: boolean): CSSProperties {
  return {
    background: active ? "#2a3340" : "transparent",
    color: active ? "#eaeef3" : "#8b94a0",
    border: "none",
    borderBottom: active ? "2px solid #8ecae6" : "2px solid transparent",
    padding: "6px 14px",
    cursor: "pointer",
    fontFamily: "monospace",
    fontSize: 13,
  };
}

function Root() {
  const [tab, setTab] = useState<Tab>("logs");
  return (
    <div style={rootStyle}>
      <div data-tauri-drag-region style={tabBarStyle}>
        <button style={tabButton(tab === "logs")} onClick={() => { setTab("logs"); }}>
          Logs
        </button>
        <button style={tabButton(tab === "stats")} onClick={() => { setTab("stats"); }}>
          Statistics
        </button>
        <button style={tabButton(tab === "hdr")} onClick={() => { setTab("hdr"); }}>
          HDR
        </button>
      </div>
      <div style={viewStyle}>
        {tab === "logs" ? <App /> : tab === "stats" ? <Statistics /> : <HdrTuning />}
      </div>
    </div>
  );
}

const rootStyle: CSSProperties = { display: "flex", flexDirection: "column", height: "100vh" };
const tabBarStyle: CSSProperties = {
  display: "flex",
  gap: 2,
  background: "#161b22",
  borderBottom: "1px solid #20262e",
  flex: "0 0 auto",
};
const viewStyle: CSSProperties = { flex: "1 1 auto", minHeight: 0 };

const container = document.getElementById("root");
if (container) {
  createRoot(container).render(
    <StrictMode>
      <Root />
    </StrictMode>,
  );
}
