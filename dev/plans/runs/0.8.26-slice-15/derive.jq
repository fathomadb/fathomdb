def nr($p): sort | .[((length * $p | ceil) - 1)];
def stats: {
  n: length,
  p50: nr(0.50),
  p95: nr(0.95),
  p99: nr(0.99),
  minimum: min,
  maximum: max
};
def median: sort | .[(length / 2 | floor)];
def mean: add / length;
def iqr: sort | {q1: nr(0.25), q3: nr(0.75)};
def five_summary:
  . as $values
  | [range(0; 5) as $a
      | range(0; 5) as $b
      | range(0; 5) as $c
      | range(0; 5) as $d
      | range(0; 5) as $e
      | [$values[$a], $values[$b], $values[$c], $values[$d], $values[$e]]
      | median] as $bootstrap
  | {
      campaign_ratios: $values,
      median_ratio: ($values | median),
      iqr: ($values | iqr),
      deterministic_exhaustive_bootstrap_95: {
        low: ($bootstrap | nr(0.025)),
        high: ($bootstrap | nr(0.975)),
        resamples: ($bootstrap | length)
      }
    };
def paired_ratio($treatment; $control):
  [.campaigns[] | ((.[$treatment] | nr(0.50)) / (.[$control] | nr(0.50)))]
  | five_summary;
def arm($name): [.campaigns[] | .[$name] | stats];

. as $root
| [range(0; 5) as $campaign
    | ($root.writer_campaigns[] | select(.campaign == $campaign and .mode == "alone")) as $alone
    | ($root.writer_campaigns[] | select(.campaign == $campaign and .mode == "graph")) as $graph
    | ($root.writer_campaigns[]
        | select(.campaign == $campaign and .mode == "hydrated_graph_plus_point_resolution")) as $composite
    | {
        campaign: $campaign,
        graph_throughput_ratio: ($graph.throughput_per_s / $alone.throughput_per_s),
        graph_p99_ratio: (($graph.latencies_us | nr(0.99)) / ($alone.latencies_us | nr(0.99))),
        composite_throughput_ratio: ($composite.throughput_per_s / $alone.throughput_per_s),
        composite_p99_ratio: (($composite.latencies_us | nr(0.99)) / ($alone.latencies_us | nr(0.99)))
      }] as $writers
| [range(0; 5) as $campaign
    | ($root.rss_campaigns[] | select(.campaign == $campaign and .mode == "control")) as $control
    | ($root.rss_campaigns[] | select(.campaign == $campaign and .mode == "treatment")) as $treatment
    | {
        campaign: $campaign,
        control_bytes: $control.peak_rss_delta_bytes,
        treatment_bytes: $treatment.peak_rss_delta_bytes,
        ratio: ($treatment.peak_rss_delta_bytes / $control.peak_rss_delta_bytes)
      }] as $rss
| ($root.campaigns | map(.control_50 | mean) | median) as $control50
| ($root.campaigns | map(.hydrated_50 | mean) | median) as $hydrated50
| ($hydrated50 - $control50) as $increment50
| {
    schema_version: 1,
    estimator: "nearest-rank; medians across five campaigns; exhaustive deterministic five-of-five bootstrap",
    unit: "microseconds unless named otherwise",
    graph: {
      one_result: {
        control: ($root | arm("control_1")),
        treatment: ($root | arm("hydrated_1")),
        paired_p50_ratio: ($root | paired_ratio("hydrated_1"; "control_1"))
      },
      fifty_results: {
        control: ($root | arm("control_50")),
        treatment: ($root | arm("hydrated_50")),
        paired_p50_ratio: ($root | paired_ratio("hydrated_50"; "control_50"))
      },
      concurrent_one_result: {
        control: ($root | arm("concurrent_control_8x200")),
        treatment: ($root | arm("concurrent_hydrated_8x200")),
        paired_p50_ratio: ($root | paired_ratio("concurrent_hydrated_8x200"; "concurrent_control_8x200"))
      },
      concurrent_fifty_results: {
        control: ($root | arm("concurrent_control_50_8x200")),
        treatment: ($root | arm("concurrent_hydrated_50_8x200")),
        paired_p50_ratio: ($root | paired_ratio("concurrent_hydrated_50_8x200"; "concurrent_control_50_8x200"))
      },
      ten_thousand_work_units_fifty_results: {
        control: ($root | arm("control_10k_work")),
        treatment: ($root | arm("hydrated_10k_work")),
        p99_is_descriptive: true,
        paired_p50_ratio: ($root | paired_ratio("hydrated_10k_work"; "control_10k_work"))
      }
    },
    point: {
      node_1k: ($root | arm("point_node_1k")),
      edge_1k: ($root | arm("point_edge_1k")),
      concurrent_1k: ($root | arm("concurrent_point_8x200")),
      node_100k: ($root | arm("point_node_100k")),
      edge_100k: ($root | arm("point_edge_100k")),
      memex_mixed_fifty: ($root | arm("memex_batch_50")),
      undersampled_p99_is_descriptive: ["node_100k", "edge_100k", "memex_mixed_fifty"]
    },
    writer: {
      campaigns: $writers,
      p99_ratios_are_descriptive: true,
      p99_samples_per_arm_are_below: 1000,
      p99_sample_count_range: {
        minimum: ($root.writer_campaigns | map(.latencies_us | length) | min),
        maximum: ($root.writer_campaigns | map(.latencies_us | length) | max)
      },
      sample_windows: ($root.writer_campaigns
        | map({campaign, mode, elapsed_us, foreground_operations,
          successful_background_ops, throughput_per_s})),
      successful_background_ops: ($root.writer_campaigns
        | map(select(.mode != "alone")
          | {campaign, mode, successful_background_ops})),
      graph_throughput_ratio: ($writers | map(.graph_throughput_ratio) | five_summary),
      graph_p99_ratio: ($writers | map(.graph_p99_ratio) | five_summary),
      hydrated_graph_plus_point_resolution_throughput_ratio:
        ($writers | map(.composite_throughput_ratio) | five_summary),
      hydrated_graph_plus_point_resolution_p99_ratio:
        ($writers | map(.composite_p99_ratio) | five_summary)
    },
    rss: {
      campaigns: $rss,
      treatment_control_ratio: ($rss | map(.ratio) | five_summary)
    },
    erasure: {
      erase_idle: ($root.erasure_campaigns | map(select(.operation == "erase") | .idle_us) | stats),
      erase_held: ($root.erasure_campaigns | map(select(.operation == "erase") | .held_us) | stats),
      excise_idle: ($root.erasure_campaigns | map(select(.operation == "excise") | .idle_us) | stats),
      excise_held: ($root.erasure_campaigns | map(select(.operation == "excise") | .held_us) | stats),
      held_outcomes: ($root.erasure_campaigns | group_by(.held_outcome) | map({outcome: .[0].held_outcome, count: length}))
    },
    carriers: {
      control_one_bytes: $root.campaigns[0].control_response_bytes,
      sidecar_one_bytes: $root.campaigns[0].sidecar_1_bytes,
      inline_one_bytes: $root.campaigns[0].inline_1_bytes,
      control_fifty_bytes: $root.campaigns[0].control_response_50_bytes,
      sidecar_fifty_bytes: $root.campaigns[0].sidecar_50_bytes,
      inline_fifty_bytes: $root.campaigns[0].inline_50_bytes
    },
    workload_model_fifty_result_expected_mean: [0, 0.01, 0.1, 0.5, 1]
      | map({
          evidence_request_fraction: .,
          option_a_expected_us: ($control50 + (. * $increment50)),
          option_b_expected_us: $hydrated50
        })
  }
