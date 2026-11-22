use crate::policy::{Classification, Policy};

#[test]
fn launch_defaults_flags_and_independent_overlap_gates() {
    assert_eq!(Policy::parse(&[]).unwrap(), (Policy::default(), None));
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let mut args = vec![];
                for (flag, enabled) in
                    [("--no-show-inactive", n), ("--show-erased", e), ("--show-payloads", t)]
                {
                    if enabled {
                        args.extend([flag.to_string(), flag.to_string()]);
                    }
                }
                args.extend(["--".into(), "-a database.sqlite".into()]);
                let (policy, path) = Policy::parse(&args).unwrap();
                assert_eq!(path.as_deref(), Some("-a database.sqlite"));
                assert_eq!(
                    policy,
                    Policy { no_show_inactive: n, show_erased: e, show_payloads: t }
                );
                for i in [false, true] {
                    for erased in [false, true] {
                        for telemetry in [false, true] {
                            let class =
                                Classification { inactive: i, erased, telemetry, known: true };
                            assert_eq!(
                                policy.permits(class),
                                (!i || !n) && (!erased || e) && (!telemetry || t)
                            );
                        }
                    }
                }
                assert!(!policy.permits(Classification {
                    inactive: false,
                    erased: false,
                    telemetry: false,
                    known: false
                }));
            }
        }
    }
    for args in [
        vec!["--show-erased=true"],
        vec!["--show-inactive"],
        vec!["a", "b"],
        vec!["--show-payloads", "false", "db"],
    ] {
        assert!(Policy::parse(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
    }
}
