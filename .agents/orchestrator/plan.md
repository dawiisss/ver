# Orchestrator Master Plan

## Overview
Rewrite the "beautiful-goodall" (VER - Very Easy Remote) connection manager application in Rust. The project will transition from Python/GTK4/Libadwaita/C-extension to a pure Rust application leveraging `gtk4-rs`, `libadwaita`, `serde`, and `vnc-rs`.

## Strategy
Following the Project Orchestrator Pattern:
1. **Phase 0: Survey & Specification Extraction**
   Spawn 3 parallel Explorers:
   - Explorer 1 (`.agents/explorer_survey_1`): Map Python app architecture, GTK4 UI layout, component hierarchy.
   - Explorer 2 (`.agents/explorer_survey_2`): Inspect existing connections JSON schema, sample files, field requirements for serde compatibility.
   - Explorer 3 (`.agents/explorer_survey_3`): Examine VNC C extension usage, frame buffer decoding expectations, mouse/keyboard event handling, and RDP (`xfreerdp3`) / SSH execution parameters.
2. **Phase 1: Feature Inventory & Milestone Decomposition (`PROJECT.md`)**
   Synthesize survey findings into `PROJECT.md` at root. Define 4 core milestones (R1-R4) and cross-module interface contracts.
3. **Phase 2: Dual Track Execution**
   - **E2E Testing Track**: Spawn E2E Testing Orchestrator / workers to create opaque-box E2E test suite (Tiers 1-4: Feature coverage, boundary cases, combinations, real-world application workloads). Publishes `TEST_READY.md`.
   - **Implementation Track**:
     - Milestone 1 (R1): Rust Skeleton & Serde Data Models
     - Milestone 2 (R2): Connection Manager UI (GTK4 / Libadwaita)
     - Milestone 3 (R3): Native VNC Client Widget (`vnc` crate) with rendering & input propagation
     - Milestone 4 (R4): RDP (`xfreerdp3`) & SSH session integration
     - Final Milestone: Pass 100% E2E tests + Tier 5 Adversarial Coverage Hardening
4. **Phase 3: Verification & Sentinel Reporting**
   Comprehensive verification by Reviewers, Challengers, and Forensic Auditor (`teamwork_preview_auditor`). Deliver completion report to Sentinel upon 100% clean verification.
