# Desktop App UI Improvements - Comprehensive List

**Date:** 2026-06-06
**Context:** QualiaDB Tauri + React/Vite Desktop Application

This document provides a detailed, categorized list of missing or underdeveloped UI features, components, and user experience improvements.

The goal is to make the desktop app feel complete, responsive, and professional, with strong user feedback and logical workflows.

---

## 1. Global App Shell & Navigation

### Missing / Weak Areas
- **Persistent Sidebar Navigation**
  - Clear section icons + labels (Dashboard, Resources, Downloads, Graph, Health, Settings)
  - Collapsible sidebar
  - Active route highlighting

- **Top Bar / Header**
  - App title + version
  - Global search bar (search across resources, data, settings)
  - User / Profile indicator (or "Sovereign Mode" badge)
  - Connection status / Backend health indicator
  - Quick actions (e.g., "Sync Catalog", "Import Data")

- **Status Bar (Bottom)**
  - Current graph stats (nodes, edges, memory usage)
  - Active downloads / background tasks
  - Notifications count
  - Offline / Sovereign mode indicator

- **Command Palette** (Cmd/Ctrl + K)
  - Quick navigation and actions

## 2. Resource Catalog Browser (New Feature)

### Core Views Needed
- **Main List View**
  - Tabs or segmented control: LLMs | Ontologies | SPARQL Endpoints
  - Search input + filters (by tag, size, license, domain, last verified)
  - Sort options (size, name, last verified, popularity)
  - Card or table layout with key metadata

- **Resource Detail Panel / Modal**
  - Full metadata display
  - Download / Import button with options
  - Preview (for small ontologies)
  - License warning / compatibility notes
  - "Add to local vault" action

- **User Feedback**
  - Toast notifications on successful download / import
  - Progress bar for large downloads
  - Error states with clear messages
  - "Already downloaded" status indicator

- **Advanced**
  - Bulk selection + actions
  - Custom resource upload form
  - Filter presets ("Edge-friendly only", "Health domain", etc.)

## 3. Downloads & Persistence Manager

### Current Gaps
- No clear view of active / completed downloads
- No progress indicators for large files (models, ontologies)
- Limited feedback when downloads fail or succeed

### Needed Features
- **Downloads Queue / History View**
  - List of active, queued, completed, and failed downloads
  - Progress bars + speed / ETA
  - Pause / Cancel / Retry actions
  - Verification status (checksum)

- **Storage Overview**
  - Disk usage by category (LLMs, Ontologies, Graphs, Other)
  - "Clean up" suggestions

- **User Feedback**
  - Toast + persistent notification center for background tasks
  - Success / failure banners

## 4. Data / Graph Explorer

- Better visual representation of the Super-Quin graph
- Search and filter within loaded data
- Node / edge detail inspector
- Export options (Turtle, JSON-LD, etc.)
- Query builder interface (basic SPARQL or visual)

## 5. Health & Wellbeing Views (Wellfair Integration)

- Dashboard with key metrics (from Samsung Health, sleep, etc.)
- Forms for manual data entry / correction
- Visualization components (charts, 3D biometric views if planned)
- Privacy / consent indicators

## 6. User Feedback & Notifications System

### Currently Weak
- Very limited feedback from backend operations
- No toast system
- Poor error communication

### Required Components
- **Toast Notification System**
  - Success, Error, Warning, Info variants
  - Auto-dismiss + manual close
  - Action buttons in toasts (e.g., "View Details")

- **Notification Center / Drawer**
  - Persistent list of past notifications
  - Mark as read / Clear all

- **Loading States**
  - Skeleton loaders for lists
  - Global loading overlay for heavy operations
  - Inline loading spinners on buttons

- **Error Handling UI**
  - User-friendly error messages
  - "Retry" buttons where appropriate
  - Detailed error modal for developers (collapsible)

- **Form Validation Feedback**
  - Inline validation errors
  - Success states on form submission

## 7. Forms & Data Entry

- Consistent form components (inputs, selects, checkboxes, textareas)
- Reusable form wrapper with validation
- File upload components (with drag & drop)
- Multi-step wizards (e.g., Import Ontology wizard)
- Confirmation dialogs for destructive actions

## 8. Settings & Configuration

- Dedicated Settings page
- Sections: General, Resource Catalog, Privacy, Downloads, Graph, Advanced
- Ability to edit `config/resources.yaml` through the UI
- Toggle for "Allow external catalog refresh"
- Theme / appearance settings

## 9. Accessibility & Polish

- Keyboard navigation support
- Screen reader friendly labels
- Consistent spacing, typography, and color system
- Responsive design (even if primarily desktop)
- Empty states with helpful guidance
- Onboarding / first-run experience

## 10. Backend <-> Frontend Communication

### Missing Patterns
- Proper event system from Rust backend to frontend
- Real-time progress updates for long operations
- Structured error responses

### Recommended
- Use Tauri's event system or a channel for progress / notifications
- Define clear TypeScript interfaces for all backend responses
- Add loading + error states to all async operations

---

## Prioritization Suggestion

1. **Resource Catalog UI** (since we're actively building the backend for it)
2. **Notification / Toast system** (foundational for good UX)
3. **Downloads view with progress**
4. **Global app shell improvements** (sidebar, header, status bar)
5. **Settings page**

---

**Next Action:** Choose a section to start implementing (e.g., Resource Catalog browser or Toast system).