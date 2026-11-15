"""One explicitly selected bounded M calibration, separate from primary admission."""

CELL_ID = "M-R-1-1-warn"
CALIBRATION_ID = "M-CAL:" + CELL_ID


def selected_cell(cell_id):
    """Return the unchanged selected workload under its separate calibration ID."""
    return None


def admit(profile, ledger, retained_bytes, limits):
    """Reserve positive remaining calibration allowances on the original shared budget."""
    return None
