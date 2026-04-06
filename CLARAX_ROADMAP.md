# ClaraX — Modern Rust-Python Bindings for Production

**Author & Sole Maintainer:** Abdulwahed Mansour
**Repository:** github.com/abdulwahed-sweden/clarax
**License:** MIT
**Version:** 1.0.1
**Status:** Production — Published on PyPI and crates.io

---

## What Is ClaraX

ClaraX is a production-focused fork of PyO3 0.28.3, the Rust-Python binding library.

It strips away legacy compatibility layers and delivers a clean, high-performance bridge between Rust and modern Python. Targets CPython 3.11+ exclusively, promotes async as first-class, and removes support for alternative interpreters.

### Key Differences from PyO3

| Area | PyO3 0.28.x | ClaraX |
|------|-------------|--------|
| Python minimum | 3.8 | **3.11** |
| Async support | Feature flag | **Always enabled** |
| CPython only | No (PyPy, GraalPy) | **Yes** |
| Deprecated APIs | Accumulated | **Removed** |
| Free-threading (3.14t) | Supported | **Supported** |
| Published names | `pyo3-*` | `clarax-*` |

## Architecture

```
clarax (Rust-Python bindings, PyO3 0.28.3 fork)
  |
  +-- clarax-core (framework-agnostic serialization + validation)
  |     Schema, Field, serialize_many, validate_many
  |     validate_names_batch, validate_ids_batch, batch_stats
  |
  +-- clarax-django (Django REST Framework integration)
        ModelSchema, RustSerializerMixin, serialize_batch
        serialize_values_list, clarax_doctor
```

## Published Packages

| Package | Registry | Description |
|---|---|---|
| clarax-core | PyPI + crates.io | Standalone serialization and validation |
| clarax-django | PyPI + crates.io | Django/DRF integration layer |
| clarax | crates.io | Core Rust-Python bindings |
| clarax-ffi | crates.io | CPython C-API declarations |
| clarax-macros | crates.io | Proc macros (#[pyclass], #[pyfunction]) |
| clarax-build-config | crates.io | Build configuration |
| clarax-macros-backend | crates.io | Code generation backend |

## Performance Summary

| Workload | Speedup |
|---|---|
| DRF serialize 1K instances | **33x** |
| DRF validate 1K instances | **50x** |
| Batch name validation (150K) | **9.1x** |
| Pattern matching (50K IDs) | **15.5x** |
| Batch statistics (50K values) | **20.8x** |
| Real Django project (Hyra) | **2.2-2.7x** |
| Dict serialization (50K) | **2.2x** |
