# Author: Abdulwahed Mansour
"""
django_pyforge — High-performance Django integration powered by Rust.

Drop-in accelerators for Django REST Framework serializers and validators.
"""

__author__ = "Abdulwahed Mansour"
__version__ = "0.1.1"

try:
    from pyforge_django import (
        ModelSchema,
        extract_model_fields,
        serialize_batch,
        serialize_fields,
        serialize_instance,
        validate_fields,
        validate_instance,
        version,
    )
except ImportError as exc:
    raise ImportError(
        "pyforge_django native extension not found. "
        "Install with: pip install pyforge-django"
    ) from exc

__all__ = [
    "ModelSchema",
    "extract_model_fields",
    "serialize_instance",
    "serialize_batch",
    "serialize_fields",
    "validate_instance",
    "validate_fields",
]
