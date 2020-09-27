
use crate::config::config_dir;
use crate::utils::{human_readable, human_readable_p10};

use std::time::Instant;

use sysinfo::{System, SystemExt as _, ProcessorExt as _, NetworkExt, DiskExt as _};
use serde_json::Value;


pub struct CommandSharedState {
    system: System,
    
    last_cpu: Option<Instant>,
    last_mem: Option<Instant>,
    last_net: Option<Instant>,
    last_disks: Option<Instant>,
    net_update_time: f32
}

#[derive(PartialEq, Clone)]
pub struct InternalCommandCommon {
    warn: Option<f64>,
    critical: Option<f64>,
    dim: Option<f64>
}

#[derive(PartialEq, Clone)]
pub enum NetStatType {
    Download,
    Upload,
    DownloadPackets,
    UploadPackets,
    DownloadErrors,
    UploadErrors,

    DownloadTotal,
    UploadTotal,
    DownloadPacketsTotal,
    UploadPacketsTotal,
    DownloadErrorsTotal,
    UploadErrorsTotal,
}

#[derive(PartialEq, Clone)]
pub enum Command {
    None,
    Shell(String),
    Literal(String),
    Array(Vec<Command>),
    
    CPUUsage(Option<usize>, InternalCommandCommon),
    CPUFreq(Option<usize>, InternalCommandCommon),

    MemUsage(InternalCommandCommon),
    MemPercent(InternalCommandCommon),
    MemTotal(InternalCommandCommon),
    MemFree(InternalCommandCommon),
    
    SwapUsage(InternalCommandCommon),
    SwapPercent(InternalCommandCommon),
    SwapTotal(InternalCommandCommon),
    SwapFree(InternalCommandCommon),

    DiskUsage(String, InternalCommandCommon),
    DiskPercent(String, InternalCommandCommon),
    DiskTotal(String, InternalCommandCommon),
    DiskFree(String, InternalCommandCommon),
    
    NetStats(NetStatType, Option<String>, InternalCommandCommon),
    NetStatsPerSecond(NetStatType, Option<String>, InternalCommandCommon)
}

impl Command {
    pub fn from(v: Value) -> Self {
        match v {
            Value::String(s) => {
                match s.chars().find(|x| !x.is_whitespace()) {
                    Some(c) => match c {
                        '#' => Self::Literal(s.chars().skip_while(|x| x.is_whitespace() || *x == '#').collect()),
                        _ => Self::Shell(s)
                    }
                    None => Self::None
                }
            }
            Value::Array(v) => Self::Array(v.iter()
                                            .map(|s| Command::from(s.to_owned()))
                                            .collect()),
            Value::Object(obj) => {
                if let Some(Value::String(t)) = obj.get("type") {

                    let get_number = |x| match obj.get(x) {
                        Some(Value::Number(x)) => Some(x.as_f64()
                            .expect(&format!("{} must be a number", x))),
                        Some(_) =>  {panic!("{} must be a number", x)}
                        None => None
                    };

                    let core = match get_number("core") {
                        Some(x) => Some(x as _),
                        None => None
                    };

                    let common = InternalCommandCommon {
                        warn: get_number("warn"),
                        critical: get_number("critical"),
                        dim: get_number("dim")
                    };

                    let network = match obj.get("network_name") {
                        Some(Value::String(s)) => Some(s.to_owned()),
                        Some(_) => panic!("network_name must be a string"),
                        None => None
                    };

                    let mountpoint = match obj.get("mountpoint") {
                        Some(Value::String(s)) => s.to_owned(),
                        Some(_) => panic!("mountpoint must be a string"),
                        None => "ERR".to_string()
                    };

                    match &t[..] {
                        "cpu_usage" => Self::CPUUsage(core, common),
                        "cpu_freq" => Self::CPUFreq(core, common),

                        "mem_usage" => Self::MemUsage(common),
                        "mem_percent" => Self::MemPercent(common),
                        "mem_total" => Self::MemTotal(common),
                        "mem_free" => Self::MemFree(common),
                        
                        "swap_usage" => Self::SwapUsage(common),
                        "swap_percent" => Self::SwapPercent(common),
                        "swap_total" => Self::SwapTotal(common),
                        "swap_free" => Self::SwapFree(common),
                        
                        "disk_usage" => Self::DiskUsage(mountpoint, common),
                        "disk_percent" => Self::DiskPercent(mountpoint, common),
                        "disk_total" => Self::DiskTotal(mountpoint, common),
                        "disk_free" => Self::DiskFree(mountpoint, common),

                        "net_download" =>               Self::NetStatsPerSecond(NetStatType::Download, network, common),
                        "net_upload" =>                 Self::NetStatsPerSecond(NetStatType::Upload, network, common),
                        "net_download_packets" =>       Self::NetStatsPerSecond(NetStatType::DownloadPackets, network, common),
                        "net_upload_packets" =>         Self::NetStatsPerSecond(NetStatType::UploadPackets, network, common),
                        "net_download_errors" =>        Self::NetStatsPerSecond(NetStatType::DownloadErrors, network, common),
                        "net_upload_errors" =>          Self::NetStatsPerSecond(NetStatType::UploadErrors, network, common),

                        "net_download_since" =>         Self::NetStats(NetStatType::Download, network, common),
                        "net_upload_since" =>           Self::NetStats(NetStatType::Upload, network, common),
                        "net_download_packets_since" => Self::NetStats(NetStatType::DownloadPackets, network, common),
                        "net_upload_packets_since" =>   Self::NetStats(NetStatType::UploadPackets, network, common),
                        "net_download_errors_since" =>  Self::NetStats(NetStatType::DownloadErrors, network, common),
                        "net_upload_errors_since" =>    Self::NetStats(NetStatType::UploadErrors, network, common),

                        "net_download_total" =>         Self::NetStats(NetStatType::DownloadTotal, network, common),
                        "net_upload_total" =>           Self::NetStats(NetStatType::UploadTotal, network, common),
                        "net_download_packets_total" => Self::NetStats(NetStatType::DownloadPacketsTotal, network, common),
                        "net_upload_packets_total" =>   Self::NetStats(NetStatType::UploadPacketsTotal, network, common),
                        "net_download_errors_total" =>  Self::NetStats(NetStatType::DownloadErrorsTotal, network, common),
                        "net_upload_errors_total" =>    Self::NetStats(NetStatType::UploadErrorsTotal, network, common),

                        _ => {panic!("Command type '{}' not available", t)}
                    }
                }
                else {
                    panic!("'type' property of command must exist if it's an object");
                }
            }
            _ => panic!("'command' must be either a string, an object with a required value 'type' or an array of those")
        }
    }

    pub fn execute(&self, gi: &mut CommandSharedState) -> String {
        match self {
            Self::Shell(s) => {

                let mut options = run_script::ScriptOptions::new();
                options.working_directory = Some(config_dir());

                let (code, output, error) = run_script::run_script!(s, options)
                    .expect("Failed to run shell script");

                if code != 0 {
                    eprintln!("WARNING: '{}' returned {}", s, code);
                }
                if !error.chars()
                    .filter(|x| !x.is_control())
                    .eq(std::iter::empty()) {
                    
                    eprintln!("WARNING: '{}' wrote to stderr:", s);
                    eprintln!("{}", error);
                }
                output
            }
            Self::CPUUsage(core, common) => {
                gi.cpu_usage(core, common)
            }
            Self::CPUFreq(core, common) => {
                gi.cpu_freq(core, common)
            }
            Self::MemUsage(common) => {
                gi.mem_usage(common)
            }
            Self::MemPercent(common) => {
                gi.mem_percent(common)
            }
            Self::MemTotal(common) => {
                gi.mem_total(common)
            }
            Self::MemFree(common) => {
                gi.mem_free(common)
            }
            Self::SwapUsage(common) => {
                gi.swap_usage(common)
            }
            Self::SwapPercent(common) => {
                gi.swap_percent(common)
            }
            Self::SwapTotal(common) => {
                gi.swap_total(common)
            }
            Self::SwapFree(common) => {
                gi.swap_free(common)
            }
            Self::DiskUsage(mnt, common) => {
                gi.disk_usage(mnt, common)
            }
            Self::DiskPercent(mnt, common) => {
                gi.disk_percent(mnt, common)
            }
            Self::DiskTotal(mnt, common) => {
                gi.disk_total(mnt, common)
            }
            Self::DiskFree(mnt, common) => {
                gi.disk_free(mnt, common)
            }
            Self::NetStats(stat, name, common) => {
                gi.net_stats(stat, name, common)
            }
            Self::NetStatsPerSecond(stat, name, common) => {
                gi.net_stats_per_second(stat, name, common)
            }
            Self::Literal(s) => s.clone(),
            Self::Array(v) => v.iter()
                               .map(|c| c.execute(gi))
                               .collect::<Vec<String>>()
                               .join(""),
            Self::None => String::new(),
        }
    }
}

impl CommandSharedState {
    pub fn new() -> Self {
        Self {
            system: sysinfo::System::new_all(),

            last_cpu: None,
            last_mem: None,
            last_net: None,
            last_disks: None,
            
            net_update_time: f32::MAX
        }
    }
    
    fn refresh_cpu(&mut self) {
        let update = if let Some(i) = self.last_cpu {
            i.elapsed().as_millis() > 30
        }
        else {true};

        if update {
            self.system.refresh_cpu();
            self.last_cpu = Some(Instant::now()); 
        }
    }

    fn refresh_mem(&mut self) {
        let update = if let Some(i) = self.last_mem {
            i.elapsed().as_millis() > 30
        }
        else {true};

        if update {
            self.system.refresh_memory();
            self.last_mem = Some(Instant::now()); 
        }
    }

    fn refresh_net(&mut self) {
        let update = if let Some(i) = self.last_net {
            i.elapsed().as_millis() > 30
        }
        else {true};

        if update {
            self.system.refresh_networks();
            self.net_update_time = self.last_net.unwrap_or(Instant::now())
                                       .elapsed().as_millis() as f32 / 1000.0;
            self.last_net = Some(Instant::now()); 
        }
    }

    fn refresh_disks(&mut self) {
        let update = if let Some(i) = self.last_disks {
            i.elapsed().as_millis() > 30
        }
        else {true};

        if update {
            self.system.refresh_disks();
            self.last_disks = Some(Instant::now()); 
        }
    }

    fn cpu(&mut self, core: &Option<usize>) -> &sysinfo::Processor {
        self.refresh_cpu();
        
        match core {
            Some(c) => {
                let a = self.system.get_processors();
                if a.len() <= *c {panic!("CPU doesn't have core {}", c)};
                &a[*c]
            }
            None => self.system.get_global_processor_info()
        }
    }

    fn mem(&mut self) -> (u64, u64) {
        self.refresh_mem();
        (self.system.get_used_memory() * 1000, self.system.get_total_memory() * 1000)
    }

    fn swap(&mut self) -> (u64, u64) {
        self.refresh_mem();
        (self.system.get_used_swap() * 1000, self.system.get_total_swap() * 1000)
    }

    fn disk(&mut self, mnt: &String) -> Option<(u64, u64)> {
        self.refresh_disks();
        for i in self.system.get_disks() {
            if i.get_mount_point() == config_dir().join(mnt) {
                return Some((i.get_total_space() - i.get_available_space(), i.get_total_space()));
            }
        }
        None
    }

    fn cpu_usage(&mut self, core: &Option<usize>, common: &InternalCommandCommon) -> String {
        let usage = self.cpu(core).get_cpu_usage();
        format!("{}{:.0}%", common.color(usage), usage)
    }

    fn cpu_freq(&mut self, core: &Option<usize>, common: &InternalCommandCommon) -> String {
        // Getting frequency for "global processor" reports 0, use core 0 freq as a fallback
        let freq = self.cpu(&Some(core.unwrap_or(0))).get_frequency() as f32;
        format!("{}{:.2}GHz", common.color(freq), freq / 1000.0)
    }

    fn common_usage(&mut self, info: Option<(u64, u64)>, common: &InternalCommandCommon) -> String {
        match info {
            Some((usage, _total)) => common.color(usage as f64) + &human_readable(usage) + "B",
            None => "???".to_string()
        }
    }

    fn common_percent(&mut self, info: Option<(u64, u64)>, common: &InternalCommandCommon) -> String {
        match info {
            Some((usage, total)) => {
                let percent = usage as f64 / total as f64 * 100.;
                format!("{}{:.2}%", common.color(percent) ,percent)
            }
            None => "???".to_string()
        }
    }

    fn common_total(&mut self, info: Option<(u64, u64)>, common: &InternalCommandCommon) -> String {
        match info {
            Some((_usage, total)) => common.color(total as f64) + &human_readable(total) + "B",
            None => "???".to_string()
        }
    }

    fn common_free(&mut self, info: Option<(u64, u64)>, common: &InternalCommandCommon) -> String {
        match info {
            Some((usage, total)) => common.color((total - usage) as f64) + &human_readable(total - usage) + "B",
            None => "???".to_string()
        }
    }

    // Memory
    fn mem_usage(&mut self, common: &InternalCommandCommon) -> String {
        let mem = Some(self.mem());
        self.common_usage(mem, common)
    }

    fn mem_percent(&mut self, common: &InternalCommandCommon) -> String {
        let mem = Some(self.mem());
        self.common_percent(mem, common)
    }

    fn mem_total(&mut self, common: &InternalCommandCommon) -> String {
        let mem = Some(self.mem());
        self.common_total(mem, common)
    }

    fn mem_free(&mut self, common: &InternalCommandCommon) -> String {
        let mem = Some(self.mem());
        self.common_free(mem, common)
    }

    // Swap
    fn swap_usage(&mut self, common: &InternalCommandCommon) -> String {
        let swap = Some(self.swap());
        self.common_usage(swap, common)
    }

    fn swap_percent(&mut self, common: &InternalCommandCommon) -> String {
        let swap = Some(self.swap());
        self.common_percent(swap, common)
    }

    fn swap_total(&mut self, common: &InternalCommandCommon) -> String {
        let swap = Some(self.swap());
        self.common_total(swap, common)
    }

    fn swap_free(&mut self, common: &InternalCommandCommon) -> String {
        let swap = Some(self.swap());
        self.common_free(swap, common)
    }

    // Disk
    fn disk_usage(&mut self, mnt: &String, common: &InternalCommandCommon) -> String {
        let disk = self.disk(mnt);
        self.common_usage(disk, common)
    }

    fn disk_percent(&mut self, mnt: &String, common: &InternalCommandCommon) -> String {
        let disk = self.disk(mnt);
        self.common_percent(disk, common)
    }

    fn disk_total(&mut self, mnt: &String, common: &InternalCommandCommon) -> String {
        let disk = self.disk(mnt);
        self.common_total(disk, common)
    }

    fn disk_free(&mut self, mnt: &String, common: &InternalCommandCommon) -> String {
        let disk = self.disk(mnt);
        self.common_free(disk, common)
    }

    fn net_stats_raw(&mut self, stat: &NetStatType, name: &Option<String>) -> Option<u64> {
        self.refresh_net();

        let (mut total, mut present) = (0, false);
        for (netname, network) in self.system.get_networks() {
            let count = match name {
                Some(n) => n == netname,
                None => true
            };

            if count {
                total += stat.get_from(network);
                present = true;
            }
        }

        match present {
            true => Some(total),
            false => None
        }
    }

    fn net_stats(&mut self, stat: &NetStatType, name: &Option<String>, common: &InternalCommandCommon) -> String {
        let total = self.net_stats_raw(stat, name);
        match total {
            Some(t) => format!("{}{}", common.color(t as f64), stat.human_readable(t)),
            None => "???".to_string()
        }
    }

    fn net_stats_per_second(&mut self, stat: &NetStatType, name: &Option<String>, common: &InternalCommandCommon) -> String {
        let total = self.net_stats_raw(stat, name);
        match total {
            Some(t) => format!("{}{}/s", common.color(t as f32 / self.net_update_time), 
                                       stat.human_readable((t as f32 / self.net_update_time) as u64)),
            None => "???".to_string()
        }
    }
}

impl InternalCommandCommon {
    fn color(&self, n: impl Into<f64>) -> String {
        let n = n.into();
        if n >= self.critical.unwrap_or(f64::MAX) {
            // Red
            "\x1b[31m".to_owned()
        }
        else if n >= self.warn.unwrap_or(f64::MAX) {
            // Yellow
            "\x1b[33m".to_owned()
        }
        else if n >= self.dim.unwrap_or(f64::MIN) {
            // Default
            "".to_owned()
        }
        else {
            // Gray
            "\x1b[90m".to_owned()
        }
    }
}

impl NetStatType {
    fn get_from(&self, n: &impl NetworkExt) -> u64 {
        match self {
            Self::Download => n.get_received(),
            Self::Upload => n.get_transmitted(),
            Self::DownloadPackets => n.get_packets_received(),
            Self::UploadPackets => n.get_packets_transmitted(),
            Self::DownloadErrors => n.get_errors_on_received(),
            Self::UploadErrors => n.get_errors_on_transmitted(),

            Self::DownloadTotal => n.get_total_received(),
            Self::UploadTotal => n.get_total_transmitted(),
            Self::DownloadPacketsTotal => n.get_total_packets_received(),
            Self::UploadPacketsTotal => n.get_total_packets_transmitted(),
            Self::DownloadErrorsTotal => n.get_total_errors_on_received(),
            Self::UploadErrorsTotal => n.get_total_errors_on_transmitted(),
        }
    }

    fn human_readable(&self, n: u64) -> String {
        match self {
            Self::Download      => human_readable(n) + "B",
            Self::Upload        => human_readable(n) + "B",
            Self::DownloadTotal => human_readable(n) + "B",
            Self::UploadTotal   => human_readable(n) + "B",
            _ => human_readable_p10(n),
        }
    }
}
