use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
}

pub fn collect() -> SystemStats {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let cpu = sys.global_cpu_usage();
    let mem_used = sys.used_memory() / 1024 / 1024;
    let mem_total = sys.total_memory() / 1024 / 1024;
    SystemStats {
        cpu_percent: cpu,
        mem_used_mb: mem_used,
        mem_total_mb: mem_total,
    }
}
