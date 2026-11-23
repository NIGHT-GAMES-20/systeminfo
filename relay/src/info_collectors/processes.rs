use crate::types::*;
use sysinfo::{Process, System, Users};

pub fn get_top_processes(
    sys: &System,
    users: &Users,
    processes_config: ProcessesConfig,
) -> Vec<ProcessInfo> {
    let mut procs: Vec<&Process> = sys.processes().values().collect();

    procs.sort_by(|a, b| match processes_config.sort {
        SortMetric::Cpu => b.cpu_usage().total_cmp(&a.cpu_usage()),
        SortMetric::Memory => b.memory().cmp(&a.memory()),
        SortMetric::Pid => b.pid().cmp(&a.pid()),
        SortMetric::RunTime => b.run_time().cmp(&a.run_time()),
    });

    procs
        .into_iter()
        .take(processes_config.count.max(100))
        .map(|p| {
            let user = p
                .user_id()
                .and_then(|uid| users.get_user_by_id(uid))
                .map(|u| u.name().to_string())
                .unwrap_or_else(|| {
                    p.user_id()
                        .map(|uid| uid.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                });

            // Join command + arguments; fallback to process name for kernel threads
            let cmd_joined = p
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");

            let command = if cmd_joined.is_empty() {
                format!("[{}]", p.name().to_string_lossy())
            } else {
                cmd_joined
            };

            let mem_bytes = p.memory();

            ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                user,
                command,
                cpu_usage: p.cpu_usage(),
                memory_bytes: mem_bytes,
                memory_mb: (mem_bytes as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0,
                status: p.status().to_string(),
                run_time_secs: p.run_time(),
            }
        })
        .collect()
}
