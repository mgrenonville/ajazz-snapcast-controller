<!--
Sync Impact Report - Constitution Update
=========================================
Version: 1.0.0 → Initial creation
Date: 2025-12-12

Changes Made:
- Initial constitution created for Ajazz Snapcast Controller project
- 3 core principles defined focused on rapid prototyping
- Principle I: Speed Over Perfection - prioritize working solutions over comprehensive testing
- Principle II: Incremental Delivery - deliver value in small, testable increments
- Principle III: Simplicity First - avoid premature optimization and over-engineering
- Governance section established with amendment procedures
- Templates validated for consistency

Templates Status:
✅ plan-template.md - Constitution Check section aligns with new principles
✅ spec-template.md - User story prioritization and independent testing requirements align
✅ tasks-template.md - Phase-based delivery aligns with incremental delivery principle
✅ All command files - No agent-specific references requiring updates

Follow-up TODOs:
- None - all placeholders filled
-->

# Ajazz Snapcast Controller Constitution

## Core Principles

### I. Speed Over Perfection

**Prototype first, refine later.** This project prioritizes rapid iteration and working solutions over comprehensive test coverage or perfect architecture. Get functionality working quickly, validate with users or stakeholders, then iterate based on feedback.

**Rationale**: For rapid prototyping projects, speed of delivery and ability to pivot based on feedback are more valuable than extensive upfront testing or perfect code. Tests are optional unless explicitly requested or when preparing for production deployment.

### II. Incremental Delivery

**Ship value in small, independent increments.** Break features into independently deliverable user stories (P1, P2, P3). Each story must be testable and demonstrable on its own. Deliver the highest-priority story first as an MVP, then add subsequent stories based on validated learning.

**Rationale**: Small, frequent deliveries enable faster feedback loops and reduce risk. Independent stories allow pivoting or stopping work without losing delivered value. This aligns with lean startup and agile principles optimized for rapid prototyping.

### III. Simplicity First

**Choose the simplest solution that works.** Avoid premature optimization, unnecessary abstractions, or frameworks when simpler alternatives exist. Start with direct implementations; add complexity only when specific problems demand it. No feature flags, backwards-compatibility layers, or "future-proofing" unless required by current needs.

**Rationale**: Complexity is the enemy of speed. Simple code is faster to write, easier to change, and clearer to understand. Over-engineering wastes time on hypothetical futures that may never arrive. YAGNI (You Aren't Gonna Need It) is a guiding principle.

## Development Workflow

### Feature Lifecycle

1. **Specification** (`/speckit.specify`): Define feature as prioritized user stories with clear acceptance criteria
2. **Planning** (`/speckit.plan`): Research technical context, identify blocking dependencies, choose simple approaches
3. **Task Generation** (`/speckit.tasks`): Break feature into ordered, independently testable tasks organized by user story
4. **Implementation** (`/speckit.implement`): Execute tasks incrementally, validating each story before moving to next priority
5. **Clarification** (`/speckit.clarify`): Ask targeted questions when specifications are underspecified; encode answers back into spec
6. **Analysis** (`/speckit.analyze`): Perform consistency checks across spec, plan, and tasks after generation

### Quality Standards

- **Tests**: Optional by default. Include only when explicitly requested or when preparing production-ready features
- **Documentation**: Focus on quickstart guides and contracts (API/interface definitions). Avoid comprehensive documentation until feature stabilizes
- **Code Review**: Lightweight reviews focused on logic correctness and major issues, not style or minor optimizations
- **Refactoring**: Defer until pain points emerge or feature set stabilizes

## Governance

### Amendment Process

1. **Proposal**: Document proposed changes to principles, workflow, or standards
2. **Validation**: Ensure changes align with rapid prototyping goals and won't slow velocity
3. **Update**: Modify constitution and increment version following semantic versioning
4. **Propagation**: Update affected templates (plan, spec, tasks) and command workflows to reflect changes
5. **Communication**: Brief team on changes and updated expectations

### Version Policy

Constitution follows semantic versioning (MAJOR.MINOR.PATCH):
- **MAJOR**: Fundamental principle changes or removals (e.g., shifting from rapid prototyping to production-first)
- **MINOR**: New principle additions or significant workflow changes
- **PATCH**: Clarifications, wording improvements, or non-semantic refinements

### Compliance

- All feature work MUST follow the defined lifecycle (specify → plan → tasks → implement)
- Constitution principles supersede individual preferences or external "best practices" that conflict with rapid prototyping goals
- Team members may propose amendments via documented proposals
- Constitution Check section in plan-template.md gates feature work: must pass before proceeding to implementation

**Version**: 1.0.0 | **Ratified**: 2025-12-12 | **Last Amended**: 2025-12-12
