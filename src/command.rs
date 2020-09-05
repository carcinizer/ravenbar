
use crate::config::config_dir;
use std::time::Instant;
use sysinfo::{System, SystemExt as _, ProcessorExt as _};


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

    fn cpu_usage(&mut self, core: &Option<usize>) -> String {
        format!("{:.0}%", self.cpu(core).get_cpu_usage())
    }

    fn cpu_freq(&mut self, core: &Option<usize>) -> String {
        // Getting frequency for "global processor" reports 0, use core 0 freq as a fallback
        format!("{:.2}MHz", self.cpu(&Some(core.unwrap_or(0))).get_frequency())
    }

    fn mem_usage(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_used_memory() * 1024) + "B"
    }

    fn mem_percent(&mut self) -> String {
        self.refresh_mem();
        format!("{:.2}%", self.system.get_used_memory() as f32 / self.system.get_total_memory() as f32 * 100.)
    }

    fn mem_total(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_total_memory() * 1024) + "B"
    }

    fn swap_usage(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_used_swap() * 1024) + "B"
    }

    fn swap_percent(&mut self) -> String {
        self.refresh_mem();
        format!("{:.2}%", self.system.get_used_swap() as f32 / self.system.get_total_swap() as f32 * 100.)
    }

    fn swap_total(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_total_swap() * 1024) + "B"
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

    format!("{:.3}{}", n as f64 / div as f64, suffix)
}

#[derive(PartialEq, Clone)]
pub enum Command{
    None,
    Shell(String),
    
    CPUUsage(Option<usize>),
    CPUFreq(Option<usize>),

    MemUsage,
    MemPercent,
    MemTotal,
    
    SwapUsage,
    SwapPercent,
    SwapTotal,
}

impl Command {
    pub fn from(s: String) -> Self {
        if s.len() == 0 {
            Self::None
        }
        else { 
            match &s[0..1] {
            "!" => {
                    let words: Vec<&str> = s[1..].split(" ").collect();
                    
                    let arg1_num = if words.len() > 1 {
                        Some(usize::from_str_radix(words[1], 10).unwrap())
                    }
                    else {None};
                    
                    match words[0] {
                        "cpu_usage" => Self::CPUUsage(arg1_num),
                        "cpu_freq" => Self::CPUFreq(arg1_num),

                        "mem_usage" => Self::MemUsage,
                        "mem_percent" => Self::MemPercent,
                        "mem_total" => Self::MemTotal,
                        
                        "swap_usage" => Self::SwapUsage,
                        "swap_percent" => Self::SwapPercent,
                        "swap_total" => Self::SwapTotal,

                        _ => {panic!("Special command not available: {}", s)}
                    }
                }
                _ => Command::Shell(s.to_owned())
            }
        }
    }

    pub fn execute(&self, gi: &mut CommandGlobalInfo) -> Result<String, run_script::ScriptError> {
        match self {
            Self::Shell(s) => {

                let mut options = run_script::ScriptOptions::new();
                options.working_directory = Some(config_dir());

                let (code, output, error) = run_script::run_script!(s, options)?;
                if code != 0 {
                    eprintln!("WARNING: '{}' returned {}", s, code);
                }
                if !error.chars()
                    .filter(|x| !x.is_control())
                    .eq(std::iter::empty()) {
                    
                    eprintln!("WARNING: '{}' wrote to stderr:", s);
                    eprintln!("{}", error);
                }
                Ok(output)
            }
            Self::CPUUsage(core) => {
                Ok(gi.cpu_usage(core))
            }
            Self::CPUFreq(core) => {
                Ok(gi.cpu_freq(core))
            }
            Self::MemUsage => {
                Ok(gi.mem_usage())
            }
            Self::MemPercent => {
                Ok(gi.mem_percent())
            }
            Self::MemTotal => {
                Ok(gi.mem_total())
            }
            Self::SwapUsage => {
                Ok(gi.swap_usage())
            }
            Self::SwapPercent => {
                Ok(gi.swap_percent())
            }
            Self::SwapTotal => {
                Ok(gi.swap_total())
            }
            Self::None => Ok(String::new()),
        }
    }
}

