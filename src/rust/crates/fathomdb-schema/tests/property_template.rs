//! Bounded property checks for the migration registry's existing contract.

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn migration_suffix_is_contiguous_and_ends_at_head(
        starting_step in 0..fathomdb_schema::SCHEMA_VERSION,
    ) {
        let suffix: Vec<u32> = fathomdb_schema::MIGRATIONS
            .iter()
            .filter(|migration| migration.step_id > starting_step)
            .map(|migration| migration.step_id)
            .collect();
        prop_assert!(!suffix.is_empty());
        prop_assert_eq!(suffix[0], starting_step + 1);
        prop_assert!(suffix.windows(2).all(|pair| pair[1] == pair[0] + 1));
        prop_assert_eq!(suffix.last().copied(), Some(fathomdb_schema::SCHEMA_VERSION));
    }
}
