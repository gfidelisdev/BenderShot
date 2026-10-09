# Single-Crate Modular Monolith Architecture

We chose a single Cargo binary crate (`enhanced-shot`) with well-isolated internal modules (`capture`, `engine`, `ui`, `indicator`, `config`) rather than a multi-crate Cargo workspace.

This maximizes build speed, simplifies dependency management across GUI and D-Bus subsystems, and reduces maintenance friction while enforcing clear internal boundaries.
