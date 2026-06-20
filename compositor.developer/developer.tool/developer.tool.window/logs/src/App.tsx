import { type CSSProperties, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  DEFAULT_FILTERS,
  type Filters,
  LEVELS,
  LEVEL_COLOR,
  type LogRecord,
  formatElapsed,
} from "./types";
import { closeWindow, dumpStore, presetStore } from "./api";

const MAX_RECORDS = 100_000;
const FLUSH_MS = 120;
const ROW_H = 18;
const OVERSCAN = 12;

/** Timeline cutoff: a microsecond value, or "live" to follow the latest record. */
type Cutoff = number | "live";

function parseRecords(json: string): LogRecord[] {
  const parsed: unknown = JSON.parse(json);
  return Array.isArray(parsed) ? (parsed as LogRecord[]) : [];
}

function parseFilters(json: string): Filters {
  const parsed: unknown = JSON.parse(json);
  if (parsed !== null && typeof parsed === "object") {
    const f = parsed as Partial<Filters>;
    return {
      levels: Array.isArray(f.levels) ? f.levels : DEFAULT_FILTERS.levels,
      crate: typeof f.crate === "string" ? f.crate : "",
      func: typeof f.func === "string" ? f.func : "",
      text: typeof f.text === "string" ? f.text : "",
    };
  }
  return DEFAULT_FILTERS;
}

export function App() {
  // Live stream buffer + an optional loaded dump that replaces the view when set.
  const [live, setLive] = useState<readonly LogRecord[]>([]);
  const [dump, setDump] = useState<readonly LogRecord[] | null>(null);
  const [paused, setPaused] = useState(false);

  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS);
  const [clearedAt, setClearedAt] = useState(0); // hide records before this elapsed (clear/restore)
  const [cutoff, setCutoff] = useState<Cutoff>("live"); // timeline position

  const [presets, setPresets] = useState<readonly string[]>([]);
  const [dumps, setDumps] = useState<readonly string[]>([]);

  const pending = useRef<LogRecord[]>([]);
  const pausedRef = useRef(paused);
  pausedRef.current = paused;

  // ── log ingestion ──────────────────────────────────────────────────────────────────
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    void listen<LogRecord>("log", (event) => {
      if (!pausedRef.current) {
        pending.current.push(event.payload);
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    const timer = window.setInterval(() => {
      if (pending.current.length === 0) {
        return;
      }
      const batch = pending.current;
      pending.current = [];
      setLive((prev) => {
        const next = prev.concat(batch);
        return next.length > MAX_RECORDS ? next.slice(next.length - MAX_RECORDS) : next;
      });
    }, FLUSH_MS);

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
      window.clearInterval(timer);
    };
  }, []);

  // ── presets / dumps lists ────────────────────────────────────────────────────────────
  function refreshPresets(): void {
    void presetStore.list().then(setPresets).catch(() => undefined);
  }
  function refreshDumps(): void {
    void dumpStore.list().then(setDumps).catch(() => undefined);
  }
  useEffect(() => {
    refreshPresets();
    refreshDumps();
  }, []);

  // ── derived view ─────────────────────────────────────────────────────────────────────
  const source = dump ?? live;
  const maxElapsed = source.length > 0 ? (source[source.length - 1]?.elapsed_micros ?? 0) : 0;
  const cutoffVal = cutoff === "live" ? maxElapsed : cutoff;
  const following = cutoff === "live";

  const crates = useMemo(
    () => Array.from(new Set(source.map((r) => r.crate_name))).sort(),
    [source],
  );

  const levelSet = useMemo(() => new Set(filters.levels), [filters.levels]);
  const filtered = useMemo(
    () =>
      source.filter(
        (r) =>
          r.elapsed_micros >= clearedAt &&
          r.elapsed_micros <= cutoffVal &&
          levelSet.has(r.level) &&
          (filters.crate === "" || r.crate_name === filters.crate) &&
          (filters.func === "" || r.function.includes(filters.func)) &&
          (filters.text === "" || r.message.includes(filters.text)),
      ),
    [source, clearedAt, cutoffVal, levelSet, filters.crate, filters.func, filters.text],
  );

  // ── virtualization ───────────────────────────────────────────────────────────────────
  const listRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(600);

  useLayoutEffect(() => {
    const el = listRef.current;
    if (!el) {
      return;
    }
    const update = (): void => {
      setViewportH(el.clientHeight);
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => {
      ro.disconnect();
    };
  }, []);

  // follow live: stick to the bottom when new records arrive
  useLayoutEffect(() => {
    const el = listRef.current;
    if (el && following) {
      el.scrollTop = el.scrollHeight;
    }
  }, [filtered, following]);

  const total = filtered.length;
  const start = Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN);
  const end = Math.min(total, Math.ceil((scrollTop + viewportH) / ROW_H) + OVERSCAN);
  const slice = filtered.slice(start, end);

  // ── handlers ─────────────────────────────────────────────────────────────────────────
  function toggleLevel(level: number): void {
    setFilters((p) => ({
      ...p,
      levels: p.levels.includes(level) ? p.levels.filter((l) => l !== level) : [...p.levels, level],
    }));
  }

  function onTimeline(value: number): void {
    setCutoff(value >= maxElapsed ? "live" : value);
  }

  function savePreset(): void {
    const name = window.prompt("Save filter preset as:");
    if (name === null || name.trim() === "") {
      return;
    }
    void presetStore.save(name.trim(), JSON.stringify(filters)).then(refreshPresets).catch(() => undefined);
  }
  function loadPreset(name: string): void {
    if (name === "") {
      return;
    }
    void presetStore.load(name).then((j) => {
      setFilters(parseFilters(j));
    }).catch(() => undefined);
  }
  function deletePreset(name: string): void {
    if (name === "" || !window.confirm(`Delete preset "${name}"?`)) {
      return;
    }
    void presetStore.remove(name).then(refreshPresets).catch(() => undefined);
  }

  function saveDump(): void {
    const name = window.prompt("Save current logs as dump:");
    if (name === null || name.trim() === "") {
      return;
    }
    void dumpStore.save(name.trim(), JSON.stringify(live)).then(refreshDumps).catch(() => undefined);
  }
  function loadDump(name: string): void {
    if (name === "") {
      return;
    }
    void dumpStore.load(name).then((j) => {
      setDump(parseRecords(j));
      setClearedAt(0);
      setCutoff("live");
    }).catch(() => undefined);
  }
  function deleteDump(name: string): void {
    if (name === "" || !window.confirm(`Delete dump "${name}"?`)) {
      return;
    }
    void dumpStore.remove(name).then(refreshDumps).catch(() => undefined);
  }

  const [presetSel, setPresetSel] = useState("");
  const [dumpSel, setDumpSel] = useState("");

  return (
    <div style={styles.app}>
      {/* title / drag / window controls */}
      <div data-tauri-drag-region style={styles.titlebar}>
        <strong data-tauri-drag-region style={{ pointerEvents: "none" }}>
          y5 log viewer
        </strong>
        {dump !== null && (
          <button type="button" style={styles.chip} onClick={() => { setDump(null); }}>
            ◀ back to live
          </button>
        )}
        <span data-tauri-drag-region style={styles.grow} />
        <button type="button" style={styles.close} onClick={() => { void closeWindow(); }}>
          ✕
        </button>
      </div>

      {/* filters */}
      <header style={styles.bar}>
        {LEVELS.map((name, level) => {
          const on = levelSet.has(level);
          return (
            <button
              key={name}
              type="button"
              onClick={() => { toggleLevel(level); }}
              style={{
                ...styles.chip,
                color: on ? "#14171c" : LEVEL_COLOR[level],
                background: on ? LEVEL_COLOR[level] : "transparent",
                borderColor: LEVEL_COLOR[level] ?? "#444",
              }}
            >
              {name.trim()}
            </button>
          );
        })}

        <select
          value={filters.crate}
          onChange={(e) => { setFilters((p) => ({ ...p, crate: e.target.value })); }}
          style={styles.input}
        >
          <option value="">all crates ({crates.length})</option>
          {crates.map((c) => (
            <option key={c} value={c}>{c}</option>
          ))}
        </select>

        {/* presets */}
        <select
          value={presetSel}
          onChange={(e) => { setPresetSel(e.target.value); loadPreset(e.target.value); }}
          style={styles.input}
          title="load a saved filter preset"
        >
          <option value="">preset…</option>
          {presets.map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
        </select>
        <button type="button" style={styles.chip} onClick={savePreset} title="save current filters as a preset">
          save preset
        </button>
        {presetSel !== "" && (
          <button type="button" style={styles.chip} onClick={() => { deletePreset(presetSel); }}>
            ✕
          </button>
        )}

        <input
          placeholder="function…"
          value={filters.func}
          onChange={(e) => { setFilters((p) => ({ ...p, func: e.target.value })); }}
          style={styles.input}
        />
        <input
          placeholder="message search…"
          value={filters.text}
          onChange={(e) => { setFilters((p) => ({ ...p, text: e.target.value })); }}
          style={{ ...styles.input, flex: 1 }}
        />
      </header>

      {/* actions: pause / clear / restore / dumps */}
      <header style={styles.bar}>
        <button type="button" style={styles.chip} onClick={() => { setPaused((p) => !p); }}>
          {paused ? "▶ resume" : "⏸ pause"}
        </button>
        <button type="button" style={styles.chip} onClick={() => { setClearedAt(maxElapsed); }} title="hide everything up to now">
          clear
        </button>
        <button type="button" style={styles.chip} onClick={() => { setClearedAt(0); }} title="show full retained history">
          restore
        </button>

        <span style={styles.sep}>dumps:</span>
        <select
          value={dumpSel}
          onChange={(e) => { setDumpSel(e.target.value); loadDump(e.target.value); }}
          style={styles.input}
        >
          <option value="">load dump…</option>
          {dumps.map((d) => (
            <option key={d} value={d}>{d}</option>
          ))}
        </select>
        <button type="button" style={styles.chip} onClick={saveDump}>save dump</button>
        {dumpSel !== "" && (
          <button type="button" style={styles.chip} onClick={() => { deleteDump(dumpSel); }}>✕</button>
        )}

        <span style={styles.count}>
          {dump !== null ? "DUMP · " : ""}{total} / {source.length}
        </span>
      </header>

      {/* timeline */}
      <div style={styles.timeline}>
        <span style={styles.time}>{formatElapsed(cutoffVal)}</span>
        <input
          type="range"
          min={0}
          max={Math.max(maxElapsed, 1)}
          value={cutoffVal}
          onChange={(e) => { onTimeline(Number(e.target.value)); }}
          style={styles.range}
        />
        <span style={{ ...styles.time, color: following ? "#7ee787" : "#6b7280" }}>
          {following ? "LIVE" : "paused@time"}
        </span>
      </div>

      {/* virtualized list */}
      <div ref={listRef} style={styles.list} onScroll={(e) => { setScrollTop(e.currentTarget.scrollTop); }}>
        <div style={{ height: total * ROW_H, position: "relative" }}>
          <div style={{ position: "absolute", top: 0, left: 0, right: 0, transform: `translateY(${String(start * ROW_H)}px)` }}>
            {slice.map((rec, i) => (
              <div key={start + i} style={styles.row}>
                <span style={styles.time}>{formatElapsed(rec.elapsed_micros)}</span>
                <span style={{ ...styles.level, color: LEVEL_COLOR[rec.level] ?? "#fff" }}>
                  {LEVELS[rec.level] ?? "?"}
                </span>
                <span style={styles.crate}>{rec.crate_name}</span>
                <span style={styles.func}>{rec.function}</span>
                <span style={styles.msg}>{rec.message}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

const styles: Record<string, CSSProperties> = {
  app: { display: "flex", flexDirection: "column", height: "100%" },
  titlebar: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "4px 8px",
    background: "#0f1216",
    borderBottom: "1px solid #2a2f38",
    userSelect: "none",
  },
  grow: { flex: 1 },
  close: {
    border: "1px solid #3a4150",
    background: "transparent",
    color: "#ff6b6b",
    borderRadius: 4,
    width: 24,
    height: 20,
    cursor: "pointer",
    font: "inherit",
  },
  bar: {
    display: "flex",
    alignItems: "center",
    gap: 6,
    padding: "5px 8px",
    background: "#1b1f26",
    borderBottom: "1px solid #2a2f38",
    flexWrap: "wrap",
  },
  chip: {
    border: "1px solid #3a4150",
    background: "transparent",
    color: "#d7dde4",
    borderRadius: 4,
    padding: "2px 8px",
    cursor: "pointer",
    font: "inherit",
  },
  input: {
    background: "#0f1216",
    color: "#d7dde4",
    border: "1px solid #2a2f38",
    borderRadius: 4,
    padding: "3px 6px",
    font: "inherit",
    minWidth: 110,
  },
  sep: { marginLeft: 8, opacity: 0.6 },
  count: { marginLeft: "auto", opacity: 0.7 },
  timeline: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "4px 10px",
    background: "#12151a",
    borderBottom: "1px solid #2a2f38",
  },
  range: { flex: 1 },
  list: { flex: 1, overflow: "auto" },
  row: {
    display: "grid",
    gridTemplateColumns: "104px 52px 260px 260px 1fr",
    gap: 10,
    height: ROW_H,
    lineHeight: `${String(ROW_H)}px`,
    padding: "0 10px",
    whiteSpace: "nowrap",
  },
  time: { color: "#6b7280" },
  level: { fontWeight: 700 },
  crate: { color: "#7aa2cf", overflow: "hidden", textOverflow: "ellipsis" },
  func: { color: "#9aa7b3", overflow: "hidden", textOverflow: "ellipsis" },
  msg: { color: "#e6ebf1", overflow: "hidden", textOverflow: "ellipsis" },
};
