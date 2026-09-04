//! Host stats via sysinfo, sampled on the blocking pool.

use std::sync::{Arc, Mutex};

use sysinfo::System;

use crate::web::info::{percent, HostInfo};

/// Samples host stats.
///
/// CPU usage is the diff since the previous sample: one `System` is kept for
/// the process lifetime (see `AppState::sys`) because sysinfo computes usage
/// between two refreshes. The blocking sysinfo calls run on the blocking
/// pool so the async runtime is never stalled.
pub async fn collect(sys: Arc<Mutex<System>>) -> HostInfo {
    tokio::task::spawn_blocking(move || sample(sys))
        .await
        .expect("host sampling task panicked")
}

/// Blocking sample of host usage stats.
fn sample(sys: Arc<Mutex<System>>) -> HostInfo {
    let mut sys = sys.lock().expect("sysinfo mutex poisoned");
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpus = sys.cpus();
    let memory_total_bytes = sys.total_memory();
    let memory_used_bytes = sys.used_memory();

    HostInfo {
        cpu_usage_percent: average_cpu_usage(cpus),
        memory_percent: percent(memory_used_bytes, memory_total_bytes),
        memory_total_bytes,
        memory_used_bytes,
    }
}

/// Mean usage across all CPUs (0 when the CPU list is empty).
fn average_cpu_usage(cpus: &[sysinfo::Cpu]) -> f32 {
    if cpus.is_empty() {
        return 0.0;
    }
    cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
}
