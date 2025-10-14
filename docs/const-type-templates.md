# Const-Driven Type Template Notes

This document summarizes the outstanding work required to specialize
function parameter and return types that depend on const parameters.

## Runtime Environment Layout

Const argument evaluation reuses the constant interpreter scratch area.
For each parameter, the environment stores the evaluated value and the
resulting type at `env_ptr + param_index * 8`.
Additional scratch slots live immediately after the environment:

- `value_ptr = env_ptr + MAX_PARAMS * 8`
- `type_ptr = value_ptr + 4`
- `stack_top_ptr = type_ptr + 4`
- `stack_base_ptr = stack_top_ptr + 4`

These offsets are required whenever we need to re-run the constant
interpreter to materialize a type template.

## Specialization Flow

1. Collect const arguments for a candidate call.
2. Determine which parameters or the return position record template
   metadata.
3. For each template, evaluate the stored expression with the collected
   const environment via `type_template_resolve_type`.
4. Use the resolved type for compatibility checks and to annotate the
   call expression.

Future work will wrap these steps in reusable helpers so call resolution
and later passes can share the same evaluation flow without duplicating
pointer arithmetic.
