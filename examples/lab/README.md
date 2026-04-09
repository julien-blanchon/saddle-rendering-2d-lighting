# Saddle Rendering 2D Lighting Lab

Crate-local lab for `saddle-rendering-2d-lighting`.

Run the interactive lab:

```bash
cargo run -p saddle-rendering-2d-lighting-lab
```

Run a scaffold E2E scenario:

```bash
cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- smoke_launch
```

Discoverable scenario names are exposed through:

- `list_scenarios()`
- `scenario_by_name()`

The current lab is intentionally a scaffold. It validates structure, diagnostics, and authoring coverage while the production render backend is still being implemented.
