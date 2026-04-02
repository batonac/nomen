import { invoke } from "@tauri-apps/api/core";
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

    useEffect(() => {
        navigateTo(currentPath);
    }, []);

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
            </header>
            {error && <div className="error-bar">{error}</div>}
            <main className="grid-container">
                <FileGrid rows={rows} onNavigate={navigateTo} />
            </main>
        </div>
    );
}

export default App;
