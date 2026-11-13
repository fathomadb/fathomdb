"""Prospective whole G-W cell estimates from accepted current-program observations."""


def forecast_write(cell, observed, remaining_ns, retained_bytes):
    """Decide next whole-cell feasibility without changing any required obligation."""
    seed_ns = 2 * observed["seed_wall_ns"]
    seed_bytes = 2 * observed["seed_bytes"]
    return {"allowed": seed_ns <= min(900 * 10**9, remaining_ns)
            and seed_bytes + retained_bytes <= 16 * 1024**3,
            "total_ns": seed_ns, "new_bytes": seed_bytes, "components": {"seed_ns": seed_ns}}
