import { useCallback } from "react";

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
    const segments = pathToSegments(currentPath);

    const handleClick = useCallback(
        (path: string) => {
            onNavigate(path);
        },
        [onNavigate]
    );

    return (
        <nav className="breadcrumb" aria-label="File path">
            {segments.map((seg, i) => (
                <span key={seg.path} className="breadcrumb-item">
                    {i > 0 && <span className="breadcrumb-sep">/</span>}
                    {i === segments.length - 1 ? (
                        <span className="breadcrumb-current">{seg.label}</span>
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
