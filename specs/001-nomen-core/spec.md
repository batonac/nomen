# Nomen — Feature Specification
**Spec ID**: 001-nomen-core  
**Status**: Draft  
**Version**: 1.0  
**Date**: 2026-03-31

---

## Overview

Nomen is a desktop file manager designed for users who need to view and edit file metadata — EXIF, IPTC, XMP, ID3 tags, and extended attributes — with the same ease and speed as navigating folders in a conventional file manager. It presents files in a high-performance grid that mirrors the "details" view familiar from Windows File Explorer, macOS Finder, and KDE Dolphin, but makes every metadata field directly editable inline, without opening a separate properties panel or external tool.

The core insight Nomen embodies is that metadata and navigation have been unnecessarily separated by every existing file manager. Nomen unifies them.

---

## Problem Statement

File managers show metadata. Metadata editors don't manage files. The user who needs to tag 200 photographs with copyright information, rating, and keywords must either use a DAM application locked to a proprietary catalogue, or use a CLI tool like ExifTool directly. Neither is acceptable for everyday use. The semantic desktop vision of Nepomuk (KDE, 2006–2014) promised this integration but was killed by performance and architectural issues before it could deliver.

Modern hardware, modern embedded databases, and modern metadata tooling make this vision achievable today.

---

## Target Users

**Primary**: Power users who manage large collections of files (photographers, archivists, researchers, journalists, developers) and need to view and edit metadata at scale.

**Secondary**: Any user who has ever wished they could edit a file's properties without leaving the file manager view.

---

## User Stories

### US-01 — Folder Navigation
**As a** file manager user,  
**I want to** navigate my filesystem using a familiar folder-traversal interface,  
**so that** I can move between directories without losing context of what I'm looking at.

**Acceptance Criteria**:
- GIVEN I am viewing a folder, WHEN I double-click a subfolder in the icon column, THEN the grid updates to show the contents of that subfolder
- GIVEN I am viewing a folder, WHEN I press Backspace or Alt+Left, THEN I navigate to the parent folder
- GIVEN I am viewing a folder, WHEN I click a segment of the breadcrumb bar, THEN I navigate to that ancestor folder
- GIVEN I am viewing a folder, WHEN I click between two breadcrumb segments, THEN a dropdown shows sibling folders at that level
- GIVEN the breadcrumb bar is focused, WHEN I press F6, THEN the breadcrumb becomes a text input where I can type a path directly
- GIVEN I type a valid path in the breadcrumb and press Enter, THEN the grid navigates to that path
- GIVEN I double-click a file (not a folder) in the icon column, THEN the file opens with the OS default application

---

### US-02 — File Grid View
**As a** file manager user,  
**I want to** see my files presented in a high-performance scrollable grid with multiple metadata columns,  
**so that** I can scan and compare file attributes across many files at once.

**Acceptance Criteria**:
- GIVEN a folder with 10,000 files, WHEN I navigate to it, THEN the grid renders without perceptible delay and scrolls at 60fps
- GIVEN the grid is populated, WHEN I click a column header, THEN the grid sorts by that column
- GIVEN the grid is sorted, WHEN I click the same column header again, THEN sort order reverses
- GIVEN the grid is populated, WHEN I drag a column header to reorder it, THEN column order updates and persists for that folder
- GIVEN the grid is populated, WHEN I drag a column header edge to resize it, THEN column width updates and persists
- GIVEN I am viewing a mixed folder (images, audio, documents), THEN each file shows values only for columns relevant to its type; other cells appear empty
- GIVEN I scroll horizontally, THEN the leftmost icon column remains frozen and visible at all times

---

### US-03 — Row Selection
**As a** file manager user,  
**I want to** select one or more files with a single click,  
**so that** I can perform file operations on the selection.

**Acceptance Criteria**:
- GIVEN the grid is populated, WHEN I single-click any cell in a row, THEN that row is selected and highlighted
- GIVEN a row is selected, WHEN I hold Shift and click another row, THEN all rows between them are selected
- GIVEN rows are selected, WHEN I hold Cmd/Ctrl and click another row, THEN that row is added to or removed from the selection
- GIVEN rows are selected, WHEN I press Cmd+A / Ctrl+A, THEN all rows are selected
- GIVEN one or more rows are selected, WHEN I right-click, THEN a context menu appears with file operations (rename, move, copy, delete, reveal in OS file manager)
- GIVEN a row is selected, WHEN I press Delete, THEN I am prompted to confirm deletion of the selected file(s)

---

### US-04 — Inline Metadata Editing
**As a** metadata-editing user,  
**I want to** edit a file's metadata directly in the grid cell,  
**so that** I can update tags without leaving the file manager view or opening a separate application.

**Acceptance Criteria**:
- GIVEN a non-icon cell is focused, WHEN I double-click it or press F2, THEN the cell enters edit mode with the current value selected
- GIVEN a cell is in edit mode, WHEN I type a new value and press Enter or Tab, THEN the value is saved and focus moves to the next cell
- GIVEN a cell is in edit mode, WHEN I press Escape, THEN the edit is cancelled and the original value is restored
- GIVEN a cell edit is committed, THEN the value is written back to the file's embedded metadata asynchronously
- GIVEN a write-back fails, THEN a non-blocking notification appears describing the failure; the displayed value reverts to the last known good value
- GIVEN a cell displays a rating field, THEN it renders as a 5-star widget, editable by clicking the desired star or typing 1–5
- GIVEN a cell displays a tag/keyword field, THEN it renders as a tag chip list, editable by clicking to add or remove chips
- GIVEN a cell displays a date field, THEN it renders with a date picker on edit

---

### US-05 — Bulk Metadata Editing
**As a** power user managing large collections,  
**I want to** apply a metadata value to multiple files simultaneously,  
**so that** I can tag batches of files without repeating the same edit for each one.

**Acceptance Criteria**:
- GIVEN one or more cells are selected in the same column, WHEN I drag the lower-right handle of the selection downward, THEN the value from the topmost selected cell is applied to all rows dragged over
- GIVEN a bulk edit is about to be applied, THEN an indicator shows "Editing N files" before confirmation
- GIVEN a bulk edit is committed, THEN all affected files have their metadata updated via a single batched ExifTool write operation
- GIVEN a bulk edit has been committed, WHEN I press Cmd+Z / Ctrl+Z, THEN all affected files have their previous metadata value restored

---

### US-06 — Column Management
**As a** metadata-editing user,  
**I want to** choose which metadata fields appear as columns in the grid,  
**so that** I can tailor the view to my current workflow.

**Acceptance Criteria**:
- GIVEN the grid is visible, WHEN I click the "+" button at the right end of the column header row, THEN a popover appears for adding a new column
- GIVEN the column popover is open, THEN I can specify: column label, metadata standard (EXIF/IPTC/XMP/ID3/xattr/User-defined), field key (with autocomplete from known ExifTool tags), and data type
- GIVEN I add a column, THEN it appears as the rightmost non-frozen column in the grid
- GIVEN I right-click a column header, THEN I can choose to hide, freeze, or configure that column
- GIVEN I have configured a column set, WHEN I save it as a named view, THEN I can recall that view from a view switcher
- GIVEN I switch to a saved view, THEN the column set, order, and widths restore exactly

---

### US-07 — Background Indexing
**As a** user opening folders with many files,  
**I want** metadata to appear quickly even for large directories,  
**so that** I am never waiting for the grid to populate.

**Acceptance Criteria**:
- GIVEN I navigate to a folder that has not been indexed, THEN system metadata (filename, size, date modified, kind) appears immediately; extended metadata columns populate progressively as ExifTool processes files
- GIVEN a folder has been indexed previously, WHEN I navigate to it, THEN all metadata appears immediately from the local index
- GIVEN files in an indexed folder have changed on disk, THEN the index is updated in the background and the grid refreshes without user action
- GIVEN indexing is in progress, THEN a subtle progress indicator is visible but does not block interaction

---

### US-08 — User-Defined Metadata
**As a** power user with domain-specific organisational needs,  
**I want to** define my own metadata fields that can be attached to any file type,  
**so that** I can track information that no standard schema provides.

**Acceptance Criteria**:
- GIVEN I create a user-defined column, THEN I can choose its write destination: embedded XMP, XMP sidecar file, or xattr
- GIVEN I write a value to a user-defined field stored as xattr, THEN the xattr is set on the file using a namespaced key (e.g., `user.nomen.fieldname`)
- GIVEN I write a value to a user-defined field stored as XMP sidecar, THEN a `.xmp` sidecar file is created or updated adjacent to the original file
- GIVEN I move a file with a user-defined xattr value to a filesystem that does not support xattr, THEN Nomen warns me that the metadata may not transfer and offers to write it to an XMP sidecar instead

---

### US-09 — File Operations
**As a** file manager user,  
**I want to** perform standard file operations from within Nomen,  
**so that** I do not need to switch to another application for basic tasks.

**Acceptance Criteria**:
- GIVEN one or more rows are selected, WHEN I choose Rename from the context menu or press F2 on the icon column, THEN the filename cell enters edit mode
- GIVEN one or more rows are selected, WHEN I choose Move from the context menu, THEN a folder picker appears and selected files are moved to the chosen destination
- GIVEN one or more rows are selected, WHEN I choose Copy from the context menu, THEN a folder picker appears and selected files are copied to the chosen destination
- GIVEN one or more rows are selected, WHEN I choose Delete from the context menu, THEN a confirmation dialog shows the number of files to be deleted before proceeding
- GIVEN I choose "Reveal in OS File Manager" from the context menu, THEN Finder/Explorer/Nautilus opens with the file selected

---

## Non-Functional Requirements

### Performance
- Grid renders folder with 10,000 files in under 500ms from indexed data
- Cell edits commit to the local index in under 16ms (one frame)
- ExifTool write-back completes within 2 seconds for single-file edits
- Bulk write-back for 100 files completes within 10 seconds
- Application cold-start to interactive in under 1 second

### Reliability
- No metadata is silently lost; all write failures surface to the user
- ExifTool daemon crash is detected and restarted automatically
- Index corruption triggers a rebuild, never a crash

### Portability
- macOS (primary target, v1.0)
- Linux (v1.0, with bundled WebView)
- Windows (v1.1)

### Accessibility
- Full keyboard navigation throughout (Tab, Arrow keys, F2, Enter, Escape, Backspace)
- All interactive elements have accessible labels
- High-contrast mode respected from OS settings

---

## Out of Scope (v1.0)

- Cloud sync or multi-device index sharing
- AI-powered auto-tagging
- Vector similarity search ("find similar files")
- Preview pane
- Plugin/extension system
- Network/remote filesystem (SMB, SFTP, S3)
- Windows support (deferred to v1.1)
- Duplicate file detection
- Batch file renaming by template

---

## Review & Acceptance Checklist

- [ ] All user stories have at least two GIVEN/WHEN/THEN acceptance criteria
- [ ] No technical implementation details appear in this document
- [ ] Out of scope section explicitly excludes ambiguous features
- [ ] Non-functional requirements are measurable
- [ ] All referenced concepts are defined or self-evident from context
- [ ] Target users are clearly described
- [ ] Problem statement clearly articulates the gap being filled
