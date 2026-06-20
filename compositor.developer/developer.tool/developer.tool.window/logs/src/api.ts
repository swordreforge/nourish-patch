import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { CompositorStats, HdrParams } from "./types";

/** One-shot diagnostics snapshot from the compositor (Statistics tab Refresh). */
export function getCompositorStats(): Promise<CompositorStats> {
  return invoke<CompositorStats>("get_compositor_stats");
}

/** Push live HDR tuning to the compositor (HDR Tuning sliders). */
export async function setHdrParams(params: HdrParams): Promise<void> {
  await invoke<null>("set_hdr_params", { params });
}

// App commands (always enabled in Tauri 2 — no capability needed). Each `<kind>` is stored
// as ~/.config/y5.compositor.developer/<kind>/<name>.json.
function makeStore(kind: "presets" | "dumps") {
  const cap = kind === "presets" ? "preset" : "dump";
  return {
    list: (): Promise<string[]> => invoke<string[]>(`list_${kind}`),
    save: (name: string, data: string): Promise<null> => invoke<null>(`save_${cap}`, { name, data }),
    load: (name: string): Promise<string> => invoke<string>(`load_${cap}`, { name }),
    remove: (name: string): Promise<null> => invoke<null>(`delete_${cap}`, { name }),
  };
}

export const presetStore = makeStore("presets");
export const dumpStore = makeStore("dumps");

export function closeWindow(): Promise<void> {
  return getCurrentWindow().close();
}
