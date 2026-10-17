//! LoCoMo evidence ids to dialog ids.

/// Split one raw LoCoMo `evidence` entry into canonical `D<session>:<turn>`
/// ids. Entries may hold several ids separated by `;` or spaces; known typos
/// (`D:11:26`, zero-padded `D30:05`) are repaired; tokens that are not an id
/// (a bare `D`) are dropped. Whether an id exists in the conversation is the
/// caller's check.
pub fn normalize_evidence(raw: &str) -> Vec<String> {
    let _ = raw;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_ids_are_split_repaired_and_filtered() {
        assert_eq!(normalize_evidence("D1:3"), ["D1:3"]);
        assert_eq!(normalize_evidence(" D10:19 "), ["D10:19"]);
        assert_eq!(normalize_evidence("D8:6; D9:17"), ["D8:6", "D9:17"]);
        assert_eq!(normalize_evidence("D9:1 D4:4 D4:6"), ["D9:1", "D4:4", "D4:6"]);
        assert_eq!(normalize_evidence("D:11:26"), ["D11:26"]);
        assert_eq!(normalize_evidence("D30:05"), ["D30:5"]);
        assert!(normalize_evidence("D").is_empty());
        assert!(normalize_evidence("").is_empty());
    }
}
