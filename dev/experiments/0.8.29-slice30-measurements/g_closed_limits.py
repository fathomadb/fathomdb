"""Closed prospective shared ceilings; historical measured cell configurations stay exact."""

DEFAULT = {"campaign_seconds": 14400, "active_bytes": 16 * 1024**3}


def validate_limits(limits):
    """Accept only explicit bounded successor totals; validation is not runtime authority."""
    result = {key: limits[key] for key in DEFAULT}
    if (type(result["campaign_seconds"]) is not int or not 14400 <= result["campaign_seconds"] <= 43200
            or type(result["active_bytes"]) is not int or not 16*1024**3 <= result["active_bytes"] <= 128*1024**3):
        raise ValueError("unsupported prospective closed shared limits")
    return result


def effective_limits(spec):
    """Use a reviewed write-only successor profile or retain the original shared limits."""
    value = spec.get("_closed_limits")
    if value is None:
        return dict(DEFAULT)
    if spec.get("preparation") != "closed-copy-v1" or spec.get("closed_mode") != "closed-copy-write-matrix":
        raise ValueError("prospective totals require the closed write-matrix review boundary")
    limits = validate_limits(value)
    review = spec.get("_reviewed_costs") or {}
    if (not isinstance(review, dict) or review.get("limits") != limits
            or (review.get("review") or {}).get("verdict") != "ACCEPT"):
        raise ValueError("prospective totals lack their matching reviewed record")
    return limits
