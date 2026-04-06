# Author: Abdulwahed Mansour
"""
django_clarax — High-performance Django integration powered by Rust.

Drop-in accelerators for Django REST Framework serializers and validators.
"""

__author__ = "Abdulwahed Mansour"
__version__ = "1.0.1"

try:
    from clarax_django import (
        ModelSchema,
        extract_model_fields,
        serialize_batch,
        serialize_fields,
        serialize_instance,
        serialize_values_list,
        validate_fields,
        validate_instance,
        version,
    )
except ImportError as exc:
    raise ImportError(
        "clarax_django native extension not found. "
        "Install with: pip install clarax-django"
    ) from exc

__all__ = [
    "ModelSchema",
    "extract_model_fields",
    "serialize_instance",
    "serialize_batch",
    "serialize_values_list",
    "serialize_fields",
    "validate_instance",
    "validate_fields",
]
