import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ColumnDefinition, FileRow } from "../shared/types";
import { COLUMN_PRESETS, type ColumnPreset } from "../shared/presets";
import { Breadcrumb } from "./Breadcrumb";
import { FileGrid } from "./FileGrid";

function getInitialPath(): string {
    return "/home";
}

function App() {
    const [rows, setRows] = useState<FileRow[]>([]);
    const [currentPath, setCurrentPath] = useState(getInitialPath());
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [indexingProgress, setIndexingProgress] = useState<{
        indexed: number;
        total: number;
        phase: string;
    } | null>(null);

    // Column management
    const [columns, setColumns] = useState<ColumnDefinition[]>([]);
    const [showColForm, setShowColForm] = useState(false);
    const [colLabel, setColLabel] = useState("");
    const [colTag, setColTag] = useState("");
    const colLabelRef = useRef<HTMLInputElement>(null);

    const navigateTo = useCallback(async (path: string) => {
        setLoading(true);
        setError(null);
        try {
            const result = await invoke<FileRow[]>("navigate_to", { path });
            setRows(result);
            setCurrentPath(path);
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
        }
    }, []);

    const openColForm = useCallback(() => {
        setShowColForm(true);
        // focus the label input on next tick
        setTimeout(() => colLabelRef.current?.focus(), 0);
    }, []);

    const handleAddColumn = useCallback(async () => {
        const tag = colTag.trim();
        const label = colLabel.trim();
        if (!label || !tag) return;
        const colon = tag.indexOf(":");
        if (colon <= 0 || colon === tag.length - 1) {
            setError('Tag must be in "Namespace:Key" format, e.g. XMP:Description');
            return;
        }
        const namespace = tag.slice(0, colon);
        const key = tag.slice(colon + 1);
        try {
            const col = await invoke<ColumnDefinition>("add_column", {
                column: { label, namespace, key, dataType: "text", writeDest: "embedded_xmp", widthPx: 160 },
            });
            setColumns((prev) => {
                // Replace if same namespace:key already exists, otherwise append.
                const idx = prev.findIndex(
                    (c) => c.namespace === col.namespace && c.key === col.key
                );
                return idx >= 0
                    ? prev.map((c, i) => (i === idx ? col : c))
                    : [...prev, col];
            });
            setShowColForm(false);
            setColLabel("");
            setColTag("");
        } catch (e) {
            setError(String(e));
        }
    }, [colLabel, colTag]);

    const handleApplyPreset = useCallback(async (preset: ColumnPreset) => {
        try {
            const all = await invoke<ColumnDefinition[]>("add_columns", {
                columns: preset.columns.map((c) => ({
                    label: c.label,
                    namespace: c.namespace,
                    key: c.key,
                    dataType: "text",
                    writeDest: "embedded_xmp",
                    widthPx: c.widthPx ?? 160,
                })),
            });
            setColumns(all);
            setShowColForm(false);
        } catch (e) {
            setError(String(e));
        }
    }, []);

    const navigateUp = useCallback(() => {
        const parent = currentPath.replace(/\/[^/]+\/?$/, "") || "/";
        navigateTo(parent);
    }, [currentPath, navigateTo]);

    // Load saved columns on mount.
    useEffect(() => {
        invoke<ColumnDefinition[]>("get_columns")
            .then(setColumns)
            .catch(console.error);
    }, []);

    // Initial navigation.
    useEffect(() => {
        navigateTo(currentPath);
    }, []);

    // Keyboard navigation.
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            if (
                e.key === "Backspace" &&
                !["INPUT", "TEXTAREA"].includes(
                    (e.target as HTMLElement)?.tagName
                )
            ) {
                e.preventDefault();
                navigateUp();
            }
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, [navigateUp]);

    // Listen for background index updates and refresh the grid.
    useEffect(() => {
        const unlistenPromise = listen<{ folderPath: string }>(
            "index-update",
            (event) => {
                if (event.payload.folderPath === currentPath) {
                    invoke<FileRow[]>("navigate_to", { path: currentPath })
                        .then(setRows)
                        .catch(console.error);
                }
            }
        );
        return () => {
            unlistenPromise.then((ul) => ul());
        };
    }, [currentPath]);

    // Listen for indexing progress.
    useEffect(() => {
        const unlistenPromise = listen<{
            folderPath: string;
            total: number;
            indexed: number;
            phase: string;
        }>("index-progress", (event) => {
            const { folderPath, total, indexed, phase } = event.payload;
            if (folderPath === currentPath) {
                if (phase === "complete") {
                    setIndexingProgress(null);
                } else {
                    setIndexingProgress({ total, indexed, phase });
                }
            }
        });
        return () => {
            unlistenPromise.then((ul) => ul());
        };
    }, [currentPath]);

    return (
        <div className="app-shell">
            <header className="app-toolbar">
                <button
                    type="button"
                    className="nav-button"
                    onClick={navigateUp}
                    title="Go up (Backspace)"
                >
                    ↑
                </button>
                <Breadcrumb
                    currentPath={currentPath}
                    onNavigate={navigateTo}
                />
                {loading && (
                    <span className="loading-indicator">Loading…</span>
                )}
                {indexingProgress && (
                    <span className="indexing-indicator" title={`${indexingProgress.phase} — ${indexingProgress.indexed}/${indexingProgress.total} files`}>
                        ⚙ {indexingProgress.indexed}/{indexingProgress.total}
                    </span>
                )}
                <button
                    type="button"
                    className="nav-button col-add-btn"
                    onClick={showColForm ? () => setShowColForm(false) : openColForm}
                    title="Add metadata column"
                >
                    ＋
                </button>
            </header>
            {showColForm && (
                <div className="col-form">
                    {COLUMN_PRESETS.map((preset) => (
                        <button
                            key={preset.name}
                            type="button"
                            className="col-btn col-btn--preset"
                            onClick={() => handleApplyPreset(preset)}
                            title={`Add ${preset.name} columns`}
                        >
                            {preset.icon} {preset.name}
                        </button>
                    ))}
                    <span className="col-form-sep" />
                    <input
                        ref={colLabelRef}
                        className="col-input"
                        placeholder="Label"
                        value={colLabel}
                        onChange={(e) => setColLabel(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleAddColumn()}
                    />
                    <input
                        className="col-input"
                        placeholder="Tag  (e.g. XMP:Description)"
                        value={colTag}
                        onChange={(e) => setColTag(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleAddColumn()}
                    />
                    <button type="button" className="col-btn col-btn--primary" onClick={handleAddColumn}>
                        Add
                    </button>
                    <button type="button" className="col-btn" onClick={() => setShowColForm(false)}>
                        Cancel
                    </button>
                </div>
            )}
            {error && <div className="error-bar" onClick={() => setError(null)}>{error}</div>}
            <main className="grid-container">
                <FileGrid
                    rows={rows}
                    columns={columns}
                    onNavigate={navigateTo}
                    onRowsChange={setRows}
                />
            </main>
        </div>
    );
}

export default App;
