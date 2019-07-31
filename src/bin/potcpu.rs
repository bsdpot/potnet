use failure::Error;
use itertools::Itertools;
use log::{error, trace, warn};
use potnet::pot::{get_running_pot_list, SystemConf};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::process::{Command as PCommand, Stdio};
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(flatten)]
    verbose: QuietVerbose,
    #[structopt(subcommand)]
    subcommand: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Show the pot virtual network status
    #[structopt(name = "show")]
    Show,
    /// Show the pot virtual network status
    #[structopt(name = "get-cpu")]
    GetCpu(GetCpuOpt),
}

#[derive(Debug, StructOpt, Copy, Clone)]
struct GetCpuOpt {
    /// Amount of CPUs needed by that pot
    #[structopt(short = "n", long = "num", default_value = "1")]
    cpu_amount: u32,
}

#[derive(Debug, Clone)]
struct CpuSet {
    cpus: Vec<u32>,
    max_cpu: u32,
}

impl Display for CpuSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.cpus.len() as u32 == self.max_cpu {
            write!(f, "not restricted")
        } else {
            let mut cpu_string = String::new();
            self.cpus.iter().for_each(|x| {
                cpu_string.push_str(&x.to_string());
                cpu_string.push(' ');
            });
            write!(f, "cpu: {}", cpu_string.trim_end())
        }
    }
}

#[derive(Debug, Clone)]
struct PotCpuConstraint {
    pot_name: String,
    cpus: u32,
}

fn get_cpuset(stdout: &[u8]) -> Option<CpuSet> {
    if let Some(ncpu) = get_ncpu() {
        if let Ok(output_string) = std::str::from_utf8(stdout) {
            if let Some(first_line) = output_string.lines().nth(0) {
                if let Some(mask) = first_line.split(':').nth(1) {
                    let v: Vec<u32> = mask
                        .split(',')
                        .map(str::trim)
                        .map(str::parse)
                        .filter(std::result::Result::is_ok)
                        .map(std::result::Result::unwrap)
                        .collect();
                    let result = CpuSet {
                        cpus: v,
                        max_cpu: ncpu,
                    };
                    return Some(result);
                } else {
                    warn!("cpuset output malformed");
                    return None;
                }
            } else {
                warn!("found no cpuset output");
                return None;
            }
        } else {
            warn!("found not UTF-8 character in the cpuset output");
            return None;
        }
    } else {
        error!("sysctl failed");
        return None;
    }
}

fn get_ncpu() -> Option<u32> {
    let output = PCommand::new("/sbin/sysctl")
        .arg("-n")
        .arg("hw.ncpu")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    if let Ok(output) = output {
        if !output.status.success() {
            warn!("failed to run sysctl");
            None
        } else if let Ok(output_string) = std::str::from_utf8(&output.stdout) {
            if let Ok(ncpu) = output_string.trim().parse::<u32>() {
                Some(ncpu)
            } else {
                warn!("failed to parse sysctl output");
                None
            }
        } else {
            warn!("failed to create a string from v[u8]");
            None
        }
    } else {
        error!("A problem occurred spawning sysctl");
        None
    }
}

fn get_cpusets(conf: &SystemConf) -> HashMap<String, CpuSet> {
    let mut result = HashMap::new();
    for pot in get_running_pot_list(conf) {
        let output = PCommand::new("/usr/bin/cpuset")
            .arg("-g")
            .arg("-j")
            .arg(&pot)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        if let Ok(output) = output {
            if !output.status.success() {
                warn!("failed to get cpuset information for pot {}", pot);
                continue;
            }
            match get_cpuset(&output.stdout) {
                Some(r) => {
                    result.insert(pot, r);
                }
                None => {
                    error!("output parsing failed");
                }
            };
        } else {
            error!("A problem occurred spawning cpuset");
            continue;
        }
    }
    result
}

fn get_potcpuconstraints(allocation: &HashMap<String, CpuSet>) -> Vec<PotCpuConstraint> {
    let mut result = Vec::new();
    for pot_name in allocation.keys() {
        if allocation[pot_name].cpus.len() as u32 == allocation[pot_name].max_cpu {
            continue;
        }
        let constraint = PotCpuConstraint {
            pot_name: pot_name.to_string(),
            cpus: allocation[pot_name].cpus.len() as u32,
        };
        result.push(constraint);
    }
    result
}

fn show(_opt: &Opt, conf: &SystemConf) {
    let pot_cpusets = get_cpusets(conf);
    for pot_name in pot_cpusets.keys() {
        let cpuset = &pot_cpusets[pot_name];
        println!("pot {} : {}", pot_name, cpuset);
    }
    let pot_allocations = get_potcpuconstraints(&pot_cpusets);
    pot_allocations.iter().for_each(|x| {
        println!("{:?}", x);
    });
}

fn get_cpu(_opt: &Opt, conf: &SystemConf, cpu_amount: u32) {
    let pot_cpusets = get_cpusets(conf);
    if let Some(ncpu) = get_ncpu() {
        if cpu_amount >= ncpu {
            return;
        }
        let mut cpu_hash_counters: HashMap<u32, u32> = HashMap::new();
        for i in 0..ncpu {
            cpu_hash_counters.insert(i, 0);
        }

        for allocations in pot_cpusets.values() {
            for cpu_num in &allocations.cpus {
                let old_value = cpu_hash_counters.remove(cpu_num).unwrap();
                cpu_hash_counters.insert(*cpu_num, old_value + 1);
            }
        }
        //dbg!(&cpu_hash_counters);
        let mut sorted_cpus = cpu_hash_counters
            .into_iter()
            .sorted_by_key(|(cpu, _allocations)| *cpu)
            .sorted_by_key(|(_cpu, allocations)| *allocations);
        //dbg!(&sorted_cpus);
        let mut cpu_string = String::new();
        for _ in 0..cpu_amount {
            let first = sorted_cpus.nth(0).unwrap().0;
            cpu_string.push_str(&first.to_string());
            cpu_string.push(',');
        }
        println!("{}", cpu_string.trim_end_matches(','));
    }
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    opt.verbose.set_log_level();
    trace!("potcpu start");

    let conf = SystemConf::new();
    if !conf.is_valid() {
        error!("No valid configuration found");
        println!("No valid configuration found");
        return Ok(());
    }
    match opt.subcommand {
        Command::Show => {
            show(&opt, &conf);
        }
        Command::GetCpu(cmd_opt) => get_cpu(&opt, &conf, cmd_opt.cpu_amount),
    }
    Ok(())
}
