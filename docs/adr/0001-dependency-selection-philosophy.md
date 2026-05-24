# Dependency Selection Philosophy

Lum prefers crates from [blessed.rs](https://blessed.rs) as a soft default. When blessed.rs doesn't cover a need, use well-known Rust crates with strong community adoption. The project targets Linux, macOS, and Windows — all dependency choices must work on all three. Always prefer a dependency over writing code yourself; the goal is minimal lines to maintain in lum.
