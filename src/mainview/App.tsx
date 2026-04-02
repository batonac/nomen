import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { FileRow } from "../shared/types";
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

    const navigateUp = useCallback(() => {
        const parent = currentPath.replace(/\/[^/]+\/?$/, "") || "/";
        navigateTo(parent);
    }, [currentPath, navigateTo]);

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
            </header>
            {error && <div className="error-bar">{error}</div>}
            <main className="grid-container">
                <FileGrid
                    rows={rows}
                    onNavigate={navigateTo}
                    onRowsChange={setRows}
                />
            </main>
        </div>
    );
}

export default App;
