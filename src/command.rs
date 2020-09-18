
use crate::config::config_dir;
use std::time::Instant;
use sysinfo::{System, SystemExt as _, ProcessorExt as _};
use serde_json::Value;


pub struct CommandGlobalInfo {
    system: System,
    
    last_cpu: Option<Instant>,
    last_mem: Option<Instant>,
}

impl CommandGlobalInfo {
    pub fn new() -> Self {
        Self {
            system: sysinfo::System::new(),

            last_cpu: None,
            last_mem: None,
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

    fn cpu_usage(&mut self, core: &Option<usize>, common: &InternalCommandCommon) -> String {
        let usage = self.cpu(core).get_cpu_usage();
        format!("{}{:.0}%", common.color(usage), usage)
    }

    fn cpu_freq(&mut self, core: &Option<usize>, common: &InternalCommandCommon) -> String {
        // Getting frequency for "global processor" reports 0, use core 0 freq as a fallback
        let freq = self.cpu(&Some(core.unwrap_or(0))).get_frequency() as f32;
        format!("{}{:.2}GHz", common.color(freq), freq / 1000.0)
    }

    fn mem_usage(&mut self, common: &InternalCommandCommon) -> String {
        let (usage, _) = self.mem();
        common.color(usage as f64) + &human_readable(usage) + "B"
    }

    fn mem_percent(&mut self, common: &InternalCommandCommon) -> String {
        let (usage, total) = self.mem();
        let percent = usage as f64 / total as f64 * 100.;
        format!("{}{:.2}%", common.color(percent) ,percent)
    }

    fn mem_total(&mut self, common: &InternalCommandCommon) -> String {
        let (_, total) = self.mem();
        common.color(total as f64) + &human_readable(total) + "B"
    }

    fn swap_usage(&mut self, common: &InternalCommandCommon) -> String {
        let (usage, _) = self.swap();
        common.color(usage as f64) + &human_readable(usage) + "B"
    }

    fn swap_percent(&mut self, common: &InternalCommandCommon) -> String {
        let (usage, total) = self.swap();
        let percent = usage as f64 / total as f64 * 100.;
        format!("{}{:.2}%", common.color(percent) ,percent)
    }

    fn swap_total(&mut self, common: &InternalCommandCommon) -> String {
        let (_, total) = self.swap();
        common.color(total as f64) + &human_readable(total) + "B"
    }
}

// TODO przeniesc do jakiegos utils.rs czy cos
pub fn human_readable(n: u64) -> String {
    let (div, suffix) : (u64, &str) = 
        if      n > (1 << 50) {(1 << 50, "Pi")}
        else if n > (1 << 40) {(1 << 40, "Ti")}
        else if n > (1 << 30) {(1 << 30, "Gi")}
        else if n > (1 << 20) {(1 << 20, "Mi")}
        else if n > (1 << 10) {(1 << 10, "Ki")}
        else {(1, "")};

    format!("{:.2}{}", n as f64 / div as f64, suffix)
}

#[derive(PartialEq, Clone)]
pub struct InternalCommandCommon {
    warn: Option<f64>,
    critical: Option<f64>,
    dim: Option<f64>
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

#[derive(PartialEq, Clone)]
pub enum Command{
    None,
    Shell(String),
    
    CPUUsage(Option<usize>, InternalCommandCommon),
    CPUFreq(Option<usize>, InternalCommandCommon),

    MemUsage(InternalCommandCommon),
    MemPercent(InternalCommandCommon),
    MemTotal(InternalCommandCommon),
    
    SwapUsage(InternalCommandCommon),
    SwapPercent(InternalCommandCommon),
    SwapTotal(InternalCommandCommon),
}

impl Command {
    pub fn from(v: Value) -> Self {
        match v {
            Value::String(s) => {
                match s.chars().find(|x| !x.is_whitespace()) {
                    Some(_) => Self::Shell(s),
                    None => Self::None
                }
            }
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

                    match &t[..] {
                        "cpu_usage" => Self::CPUUsage(core, common),
                        "cpu_freq" => Self::CPUFreq(core, common),

                        "mem_usage" => Self::MemUsage(common),
                        "mem_percent" => Self::MemPercent(common),
                        "mem_total" => Self::MemTotal(common),
                        
                        "swap_usage" => Self::SwapUsage(common),
                        "swap_percent" => Self::SwapPercent(common),
                        "swap_total" => Self::SwapTotal(common),

                        _ => {panic!("Command type '{}' not available", t)}
                    }
                }
                else {
                    panic!("'type' property of command must exist if it's an object");
                }
            }
            _ => panic!("'command' must be either a string or an object with a required value 'type'")
        }
    }

    pub fn execute(&self, gi: &mut CommandGlobalInfo) -> String {
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
            Self::SwapUsage(common) => {
                gi.swap_usage(common)
            }
            Self::SwapPercent(common) => {
                gi.swap_percent(common)
            }
            Self::SwapTotal(common) => {
                gi.swap_total(common)
            }
            Self::None => String::new(),
        }
    }
}

