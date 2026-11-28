# Visual Diagram Modeling & Rendering Pipeline

This directory holds the source models and rendered vector assets for system architecture diagrams under the ASEP framework.

---

## 1. Directory Structure

```
docs/diagrams/
├── README.md               # Pipeline documentation and usage instructions
├── rendered/               # Automatically compiled vector assets (.svg, .png)
│   └── .gitkeep
├── checkout_bce.puml       # Formal PlantUML behavioral / sequence models
└── system_topology.mmd     # Mermaid architectural flowcharts
```

---

## 2. Rendering Commands

### A. PlantUML Rendering (to SVG)
PlantUML generates scalable, pixel-perfect vector graphics (`.svg`):

```bash
# Compile all .puml files into rendered/
plantuml -tsvg docs/diagrams/*.puml -o rendered/

# Alternatively, using docker if plantuml is not installed locally:
docker run --rm -v $(pwd)/docs/diagrams:/work ghcr.io/plantuml/plantuml:latest -tsvg /work/*.puml -o /work/rendered/
```

### B. Mermaid Rendering (to SVG)
Using `@mermaid-js/mermaid-cli` (`mmdc`):

```bash
# Compile a single Mermaid source file into rendered/
npx -y @mermaid-js/mermaid-cli -i docs/diagrams/system_topology.mmd -o docs/diagrams/rendered/system_topology.svg
```

---

## 3. Embedding Diagrams into Specifications

Embed rendered diagrams into `docs/PRD.md` or `docs/TDD.md` using standard markdown image syntax:

```markdown
![Checkout BCE Sequence](diagrams/rendered/checkout_bce.svg)
```

For high-level conceptual flows that do not require external asset compilation, you may also embed dark-mode Mermaid fenced code blocks directly inline in the markdown file:

````markdown
```mermaid
flowchart LR
    Client --> API[REST Gateway] --> Service[Domain Service]
```
````

---

## 4. Modeling Standards

- **Denert's Law (1991) BCE Architecture:** Sequence diagrams on critical paths should use explicit `boundary`, `control`, and `entity` stereotypes to decouple presentation, business orchestration, and data persistence.
- **Dark Mode Friendly:** Use monochromatic or high-contrast styles (`skinparam monochrome true`, `skinparam shadowing false`) to ensure diagrams render cleanly in both dark and light Obsidian/IDE preview modes.
