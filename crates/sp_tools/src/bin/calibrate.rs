use sp_tools::calibration::{CALIBRATION_TARGETS, run_calibration};
use std::time::Instant;

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seed: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let t0 = Instant::now();
    let report = run_calibration(n, seed);
    let ms = t0.elapsed().as_millis();
    println!("{report}");
    println!("Targets (min..max) and status:");
    for (k, lo, hi) in CALIBRATION_TARGETS {
        let v = report.avg[k];
        println!("  {k:<20} {v:>8.2}  [{lo:.1}..{hi:.1}] {}", if v >= lo && v <= hi { "ok" } else { "OUT" });
    }
    println!("Elapsed: {ms} ms ({:.1} ms/game)", ms as f64 / n as f64);
}
