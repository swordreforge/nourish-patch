import { type CSSProperties, useCallback, useState } from "react";
import { setHdrParams } from "./api";
import { DEFAULT_HDR_PARAMS, type HdrParams } from "./types";

interface SliderSpec {
  readonly key: keyof HdrParams;
  readonly label: string;
  readonly min: number;
  readonly max: number;
  readonly step: number;
  readonly unit?: string;
}

// Continuous knobs (rendered as range sliders).
const SLIDERS: readonly SliderSpec[] = [
  { key: "sdr_white_nits", label: "SDR reference white", min: 50, max: 600, step: 1, unit: "nits" },
  { key: "max_nits", label: "Display max luminance", min: 100, max: 10000, step: 10, unit: "nits" },
  { key: "exposure", label: "Exposure", min: 0, max: 3, step: 0.01 },
  { key: "brightness", label: "Brightness", min: 0, max: 3, step: 0.01 },
  { key: "contrast", label: "Contrast", min: 0.5, max: 2, step: 0.01 },
  { key: "saturation", label: "Saturation", min: 0, max: 2, step: 0.01 },
  { key: "gamut", label: "Gamut (709→2020)", min: 0, max: 1, step: 0.01 },
  { key: "gamma", label: "Gamma", min: 0.5, max: 2, step: 0.01 },
];

// 0/1 toggles.
const TOGGLES: readonly { readonly key: keyof HdrParams; readonly label: string; readonly off: string; readonly on: string }[] = [
  { key: "enabled", label: "HDR encode", off: "passthrough", on: "active" },
  { key: "tone_map", label: "Highlight tone-map", off: "clip", on: "Reinhard" },
  { key: "transfer", label: "Transfer", off: "PQ", on: "HLG" },
];

export function HdrTuning() {
  const [params, setParams] = useState<HdrParams>(DEFAULT_HDR_PARAMS);
  const [error, setError] = useState<string | null>(null);

  const push = useCallback((next: HdrParams) => {
    setParams(next);
    void setHdrParams(next)
      .then(() => {
        setError(null);
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
      });
  }, []);

  const set = useCallback(
    (key: keyof HdrParams, value: number) => {
      push({ ...params, [key]: value });
    },
    [params, push],
  );

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h2 style={styles.title}>HDR Tuning</h2>
        <button style={styles.button} onClick={() => { push(DEFAULT_HDR_PARAMS); }}>
          Reset
        </button>
      </div>
      <div style={styles.note}>
        Live — requires the compositor running with <code>COMPOSITOR_HDR=1</code> on a Vulkan,
        HDR-capable output. Changes apply on the next frame.
      </div>

      {error !== null && <div style={styles.error}>⚠ {error}</div>}

      <div style={styles.toggles}>
        {TOGGLES.map((t) => {
          const on = params[t.key] >= 0.5;
          return (
            <button
              key={t.key}
              style={{ ...styles.toggle, ...(on ? styles.toggleOn : {}) }}
              onClick={() => { set(t.key, on ? 0 : 1); }}
            >
              {t.label}: {on ? t.on : t.off}
            </button>
          );
        })}
      </div>

      {SLIDERS.map((s) => (
        <div key={s.key} style={styles.row}>
          <span style={styles.label}>{s.label}</span>
          <input
            type="range"
            min={s.min}
            max={s.max}
            step={s.step}
            value={params[s.key]}
            style={styles.range}
            onChange={(e) => { set(s.key, Number(e.target.value)); }}
          />
          <span style={styles.value}>
            {params[s.key].toFixed(s.step < 1 ? 2 : 0)}
            {s.unit !== undefined ? ` ${s.unit}` : ""}
          </span>
        </div>
      ))}
    </div>
  );
}

const styles = {
  container: {
    padding: "12px 16px",
    overflowY: "auto",
    height: "100%",
    fontFamily: "monospace",
    fontSize: 13,
    color: "#d6dbe2",
  },
  header: { display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 },
  title: { margin: 0, fontSize: 16 },
  note: { color: "#8b94a0", fontSize: 12, margin: "4px 0 12px" },
  error: { color: "#ff6b6b", padding: "6px 0" },
  toggles: { display: "flex", flexWrap: "wrap", gap: 8, marginBottom: 12 },
  toggle: {
    background: "#2a3340",
    color: "#8b94a0",
    border: "1px solid #3a4452",
    borderRadius: 4,
    padding: "4px 10px",
    cursor: "pointer",
    fontFamily: "monospace",
    fontSize: 12,
  },
  toggleOn: { color: "#8ecae6", borderColor: "#8ecae6" },
  row: { display: "flex", alignItems: "center", gap: 12, padding: "3px 0" },
  label: { color: "#8ecae6", flex: "0 0 170px" },
  range: { flex: "1 1 auto" },
  value: { color: "#eaeef3", flex: "0 0 90px", textAlign: "right" },
  button: {
    background: "#2a3340",
    color: "#eaeef3",
    border: "1px solid #3a4452",
    borderRadius: 4,
    padding: "4px 12px",
    cursor: "pointer",
  },
} satisfies Record<string, CSSProperties>;
