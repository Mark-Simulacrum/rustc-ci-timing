# rust-lang/rust CI timing tracking

This repository downloads CI timing data for each builder on rust-lang/rust's
auto builds, based on the CPU usage data collection run in the background on
rust's CI, and graphs it.

To use, run:

```bash
cargo run --release # May take a while on the first run, incremental
python3 walltime.py
```

You can edit walltime.py to adjust which builders are shown (see the
`filter_display` function).
