# PyForge — Modern Rust-Python Bindings for Production

**Author & Sole Maintainer:** Abdulwahed Mansour  
**Repository:** github.com/abdulwahed-sweden/pyforge  
**License:** MIT OR Apache-2.0  
**Version:** 0.30.0  
**Status:** Active Development — Phase 1 In Progress

---

## What Is PyForge

PyForge is a modernized, production-focused fork of PyO3, the Rust-Python binding library.

PyForge strips away legacy compatibility layers and delivers a clean, high-performance bridge between Rust and modern Python. It targets CPython 3.11+ exclusively, promotes async as first-class, and removes support for alternative interpreters (PyPy, GraalPy) that add complexity without serving modern production stacks.

### Key Differences from PyO3

| Area | PyO3 0.28.x | PyForge 0.30.0 |
|------|-------------|----------------|
| Python minimum | 3.8 | **3.11** |
| Async support | `experimental-async` feature flag | **Always enabled, first-class** |
| CPython only | No (PyPy, GraalPy supported) | **Yes — CPython only** |
| Deprecated APIs | Accumulated (GILOnceCell public, old casts) | **Removed** |
| abi3 minimum | py38 | **py311** |
| `extension-module` flag | Deprecated but present | **Removed** |
| `generate-import-lib` flag | Deprecated but present | **Removed** |
| `num-complex` / `num-rational` | Included | **Removed from full feature set** |
| Published crate names | `pyo3-*` | `pyforge-*` |

---

## Changes Completed (Phase 1)

### 1. Crate Identity

All 6 workspace crates renamed:

| Old Name | New Name |
|----------|----------|
| `pyo3` | `pyforge` |
| `pyo3-ffi` | `pyforge-ffi` |
| `pyo3-macros` | `pyforge-macros` |
| `pyo3-macros-backend` | `pyforge-macros-backend` |
| `pyo3-build-config` | `pyforge-build-config` |
| `pyo3-introspection` | `pyforge-introspection` |

Author set to **Abdulwahed Mansour** across all crates.

### 2. Python 3.11+ Minimum

- `MINIMUM_SUPPORTED_VERSION` raised from `3.8` to `3.11`
- abi3 feature chain starts at `abi3-py311` (removed `abi3-py38`, `abi3-py39`, `abi3-py310`)
- Build-time assertion enforces CPython 3.11+ and rejects PyPy/GraalPy
- FFI package metadata updated: `min-version = "3.11"`
- PyPy metadata section removed from `pyforge-ffi`

### 3. Async First-Class

- Removed `experimental-async` compile-time gates from `pyfunction.rs` and `pymethod.rs`
- Removed `#[cfg(feature = "experimental-async")]` from `coroutine` module
- `experimental-async` is now included in default features
- `async fn` works out of the box with `#[pyfunction]` and `#[pymethods]`

### 4. Deprecated API Cleanup

- Removed deprecated `with_critical_section()` and `with_critical_section2()` public aliases
- Cleaned `GILOnceCell` — removed deprecated annotation, kept as internal-only (`pub(crate)`)
- Removed all `#[allow(deprecated)]` noise from `sync.rs` and `lazy_type_object.rs`
- Removed deprecated `FromPyObject` automatic derivation warning
- Removed deprecated `extension-module` feature flag
- Removed deprecated `generate-import-lib` feature flag
- Removed `num-complex` and `num-rational` from `full` feature set

---

## Development Roadmap

### Phase 1: Foundation (Current)

- [x] Rename all crates to `pyforge-*`
- [x] Set sole authorship to Abdulwahed Mansour
- [x] Raise minimum Python to 3.11
- [x] Make async first-class
- [x] Remove deprecated APIs
- [x] Enforce CPython-only at build time
- [ ] Full source rename (`pyo3` -> `pyforge` in all Rust code)
- [ ] Remove `#[cfg(PyPy)]` and `#[cfg(GraalPy)]` code paths from source
- [ ] Remove Python 3.8/3.9/3.10 compatibility shims from FFI
- [ ] Simplify CI matrix to CPython 3.11/3.12/3.13

### Phase 2: Modern API Surface

- [ ] Promote `experimental-inspect` to stable
- [ ] Add `#[pyfunction]` attribute for automatic type stub generation
- [ ] Improve error messages for async context misuse
- [ ] Add `serde_json::Value` <-> Python `dict` zero-copy path
- [ ] Add `tokio` runtime integration for `#[pyfunction] async fn`

### Phase 3: Performance

- [ ] Benchmark suite comparing PyForge vs PyO3 vs raw FFI
- [ ] Optimize `Vec<T>` <-> `list` conversion with buffer protocol
- [ ] Add bulk serialization fast-path (Rayon parallel)
- [ ] Profile and optimize `#[pyclass]` instance creation

### Phase 4: Publish

- [ ] Security audit and dependency review
- [ ] Publish `pyforge` to crates.io
- [ ] Build Python wheels via `maturin` for CPython 3.11/3.12/3.13
- [ ] Publish to PyPI: `pip install pyforge`
- [ ] Documentation site
- [ ] GitHub release with changelog

---

## Contribution Policy

**Sole Author and Maintainer:** Abdulwahed Mansour  
**GitHub:** github.com/abdulwahed-sweden

### Commit Convention

```
<type>(<scope>): <description>

Signed-off-by: Abdulwahed Mansour
```

Types: `feat`, `fix`, `refactor`, `docs`, `ci`, `perf`, `test`, `chore`  
Scopes: `core`, `ffi`, `macros`, `build`, `async`

---

## Technology Stack

| Layer | Technology | Constraint |
|-------|-----------|------------|
| Language | Rust 2021 edition | Stable toolchain, MSRV 1.83 |
| Python | CPython | 3.11+ only |
| Build | maturin | Latest |
| Serialization | serde + serde_json | Latest |
| Date/Time | chrono, jiff | Latest |
| UUID | uuid | Latest |
| Decimal | rust_decimal, bigdecimal | Latest |

---

*This document is the authoritative roadmap for the PyForge project.*  
*All work is performed by Abdulwahed Mansour.*
