
use crate::command::{CommandTrait, CommandSharedState};
use crate::config::config_dir;
use crate::utils::{human_readable, human_readable_p10};

use std::time::Instant;

use ::sysinfo::{System, SystemExt as _, ProcessorExt as _, NetworkExt, DiskExt as _};

#[derive(Clone, PartialEq)]
pub struct CPUUsageCommand(pub Option<usize>);
#[derive(Clone, PartialEq)]
pub struct CPUFreqCommand(pub Option<usize>);

#[derive(Clone, PartialEq)]
pub struct MemoryInfoCommand {
    pub ty: MemoryInfoType,
    pub val: MemoryInfoValue
}

#[derive(Clone, PartialEq)]
pub enum MemoryInfoType {
    RAM,
    Swap,
    Disk(Option<String>)
}

#[derive(Clone, PartialEq)]
pub enum MemoryInfoValue {
    Usage,
    Total,
    Free,
    Percent
}


#[derive(Clone, PartialEq)]
pub struct NetInfoCommand {
    pub name: Option<String>,
    pub ty: NetInfoType,
    pub time: NetInfoTime,
    pub val: NetInfoValue
}

#[derive(Clone, PartialEq)]
pub enum NetInfoType {
    Download,
    Upload
}

#[derive(Clone, PartialEq)]
pub enum NetInfoTime {
    Since,
    PerSecond,
    Total
}

#[derive(Clone, PartialEq)]
pub enum NetInfoValue {
    Bits,
    Bytes,
    Packets,
    Errors
}

pub struct SystemSingleton {
    system: System,
    
    last_cpu: Option<Instant>,
    last_mem: Option<Instant>,
    last_net: Option<Instant>,
    last_disks: Option<Instant>,
    net_update_time: f64
}

impl CommandTrait for CPUUsageCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        format!("{:.0}%", state.get::<SystemSingleton>(0).cpu_usage(&self.0))
    }
}

impl CommandTrait for CPUFreqCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        format!("{}Hz", human_readable_p10(state.get::<SystemSingleton>(0).cpu_freq(&self.0) as _))
    }
}

impl CommandTrait for MemoryInfoCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        if let Some((usage, total)) = state.get::<SystemSingleton>(0).mem(&self.ty) {

            match self.val {
                MemoryInfoValue::Total => human_readable(total) + "B",
                MemoryInfoValue::Usage => human_readable(usage) + "B",
                MemoryInfoValue::Free => human_readable(total - usage) + "B",
                MemoryInfoValue::Percent => format!("{:.0}%", usage as f64 / total as f64 * 100.0),
            }
        }
        else {"ERR".to_string()}
    }
}

impl CommandTrait for NetInfoCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {

        let value = state.get::<SystemSingleton>(0)
            .net(&self.name, &self.ty, &self.time, &self.val);
        if let None = value {
            return "ERR".to_string();
        }

        let (unit, sep, hr) = match self.val {
            NetInfoValue::Bits => ("b", "ps", human_readable_p10(value.unwrap())),
            NetInfoValue::Bytes => ("B", "/s", human_readable(value.unwrap())),
            _ => ("", "/s", human_readable_p10(value.unwrap()))
        };
        let suffix = if let NetInfoTime::PerSecond = self.time {sep} else {""};
        
        format!("{}{}{}", hr, unit, suffix)
    }
}

macro_rules! refresh {($last:expr, $refresh:expr) => {{

    let update = if let Some(i) = $last {
        i.elapsed().as_millis() > 30
    }
    else {true};

    if update {
        $refresh;
        $last = Some(Instant::now()); 
    }
}}}

impl SystemSingleton {

    fn refresh_cpu(&mut self) {
        refresh!(self.last_cpu, self.system.refresh_cpu())
    }
    fn refresh_mem(&mut self) {
        refresh!(self.last_mem, self.system.refresh_memory())
    }
    fn refresh_disks(&mut self) {
        refresh!(self.last_disks, self.system.refresh_disks())
    }
    fn refresh_net(&mut self) {
        if let None = self.last_net {
            self.system.refresh_networks_list();
        }
        self.net_update_time = self.last_net.unwrap_or(Instant::now())
                                   .elapsed().as_millis() as f64 / 1000.0;
        refresh!(self.last_mem, self.system.refresh_networks())
    }

    fn cpu_usage(&mut self, core: &Option<usize>) -> f32 {
        self.cpu(core).get_cpu_usage()
    }

    fn cpu_freq(&mut self, core: &Option<usize>) -> u64 {
        // Getting frequency for "global processor" reports 0, use core 0 freq as a fallback
        self.cpu(&Some(core.unwrap_or(0))).get_frequency() * 1000000
    }

    fn cpu(&mut self, core: &Option<usize>) -> &::sysinfo::Processor {
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

    fn mem(&mut self, ty: &MemoryInfoType) -> Option<(u64, u64)> {
        match ty {
            MemoryInfoType::RAM => {
                self.refresh_mem();
                Some((self.system.get_used_memory() * 1000, self.system.get_total_memory() * 1000))
            }
            MemoryInfoType::Swap => {
                self.refresh_mem();
                Some((self.system.get_used_swap() * 1000, self.system.get_total_swap() * 1000))

            }
            MemoryInfoType::Disk(mnt) => {
                self.refresh_disks();
                if let None = mnt {
                    return None;
                }

                for i in self.system.get_disks() {
                    if i.get_mount_point() == config_dir().join(mnt.as_ref().unwrap()) {
                        return Some((i.get_total_space() - i.get_available_space(), i.get_total_space()));
                    }
                }
                None
            }
        }
    }

    fn net(&mut self, name: &Option<String>, ty: &NetInfoType, time: &NetInfoTime, val: &NetInfoValue) -> Option<u64> {
        self.refresh_net();

        // Multiplier for quantity per second
        let timemul = if let NetInfoTime::PerSecond = time {
            self.net_update_time
        }
        else {1.0};

        // Multiplier for bits
        let bitmul = if let NetInfoValue::Bits = val {8} else {1};

        let (mut total, mut present) = (0, false);
        for (netname, network) in self.system.get_networks() {
            let count = match name {
                Some(n) => n == netname,
                None => true
            };

            if count {
                let a = timemul * { match time {
                    NetInfoTime::Since | NetInfoTime::PerSecond => match ty {
                        NetInfoType::Download => match val {
                            NetInfoValue::Bits | NetInfoValue::Bytes => network.get_received() * bitmul,
                            NetInfoValue::Packets => network.get_packets_received(),
                            NetInfoValue::Errors => network.get_errors_on_received(),
                        }
                        NetInfoType::Upload => match val {
                            NetInfoValue::Bits | NetInfoValue::Bytes => network.get_transmitted() * bitmul,
                            NetInfoValue::Packets => network.get_packets_transmitted(),
                            NetInfoValue::Errors => network.get_errors_on_transmitted(),
                        }
                    } 
                    NetInfoTime::Total => match ty {
                        
                        NetInfoType::Download => match val {
                            NetInfoValue::Bits | NetInfoValue::Bytes => network.get_total_received() * bitmul,
                            NetInfoValue::Packets => network.get_total_packets_received(),
                            NetInfoValue::Errors => network.get_total_errors_on_received(),
                        }
                        NetInfoType::Upload => match val {
                            NetInfoValue::Bits | NetInfoValue::Bytes => network.get_total_transmitted() * bitmul,
                            NetInfoValue::Packets => network.get_total_packets_transmitted(),
                            NetInfoValue::Errors => network.get_total_errors_on_transmitted(),
                        }
                    }
                } } as f64;

                total += a as u64;
                present = true;
            }
        }

        match present {
            true => Some(total),
            false => None
        }
    }
}

impl Default for SystemSingleton {
    
    fn default() -> Self {
        Self {
            system: System::new(),
            last_cpu: None,
            last_mem: None,
            last_net: None,
            last_disks: None,
            net_update_time: f64::MAX
        }
    }
}
