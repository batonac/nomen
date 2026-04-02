import { useCallback, useEffect, useRef, useState } from "react";

interface BreadcrumbProps {
    currentPath: string;
    onNavigate: (path: string) => void;
}

interface Segment {
    label: string;
    path: string;
}

function pathToSegments(fullPath: string): Segment[] {
    const parts = fullPath.split("/").filter(Boolean);
    const segments: Segment[] = [{ label: "/", path: "/" }];
    for (let i = 0; i < parts.length; i++) {
        segments.push({
            label: parts[i],
            path: "/" + parts.slice(0, i + 1).join("/"),
        });
    }
    return segments;
}

export function Breadcrumb({ currentPath, onNavigate }: BreadcrumbProps) {
    const [editMode, setEditMode] = useState(false);
    const [inputValue, setInputValue] = useState(currentPath);
    const inputRef = useRef<HTMLInputElement>(null);
    const segments = pathToSegments(currentPath);

    // Update input when path changes externally.
    useEffect(() => {
        if (!editMode) {
            setInputValue(currentPath);
        }
    }, [currentPath, editMode]);

    // Focus input when entering edit mode.
    useEffect(() => {
        if (editMode) {
            inputRef.current?.focus();
            inputRef.current?.select();
        }
    }, [editMode]);

    // F6 key: enter text mode from anywhere in the window.
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            if (e.key === "F6") {
                e.preventDefault();
                setEditMode(true);
            }
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, []);

    const handleClick = useCallback(
        (path: string) => {
            onNavigate(path);
        },
        [onNavigate]
    );

    const commitEdit = useCallback(() => {
        const trimmed = inputValue.trim();
        if (trimmed) {
            onNavigate(trimmed);
        }
        setEditMode(false);
    }, [inputValue, onNavigate]);

    const cancelEdit = useCallback(() => {
        setInputValue(currentPath);
        setEditMode(false);
    }, [currentPath]);

    const handleKeyDown = useCallback(
        (e: React.KeyboardEvent<HTMLInputElement>) => {
            if (e.key === "Enter") {
                e.preventDefault();
                commitEdit();
            } else if (e.key === "Escape") {
                e.preventDefault();
                cancelEdit();
            }
        },
        [commitEdit, cancelEdit]
    );

    if (editMode) {
        return (
            <nav className="breadcrumb breadcrumb--edit" aria-label="File path">
                <input
                    ref={inputRef}
                    type="text"
                    className="breadcrumb-input"
                    value={inputValue}
                    onChange={(e) => setInputValue(e.target.value)}
                    onKeyDown={handleKeyDown}
                    onBlur={cancelEdit}
                    aria-label="Navigate to path"
                />
            </nav>
        );
    }

    return (
        <nav
            className="breadcrumb"
            aria-label="File path"
            tabIndex={0}
        >
            {segments.map((seg, i) => (
                <span key={seg.path} className="breadcrumb-item">
                    {i > 0 && <span className="breadcrumb-sep">/</span>}
                    {i === segments.length - 1 ? (
                        <span
                            className="breadcrumb-current"
                            onDoubleClick={() => setEditMode(true)}
                            title="Double-click or press F6 to edit"
                        >
                            {seg.label}
                        </span>
                    ) : (
                        <button
                            type="button"
                            className="breadcrumb-link"
                            onClick={() => handleClick(seg.path)}
                        >
                            {seg.label}
                        </button>
                    )}
                </span>
            ))}
        </nav>
    );
}
