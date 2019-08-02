use failure::{Error, Fail};
use itertools::Itertools;
use log::{error, info, trace, warn};
use potnet::pot::{get_running_pot_list, SystemConf};
use std::collections::HashMap;
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
    /// Propose a new allocation layout if needed
    #[structopt(name = "rebalance")]
    Rebalance,
}

#[derive(Debug, StructOpt, Copy, Clone)]
struct GetCpuOpt {
    /// Amount of CPUs needed by that pot
    #[structopt(short = "n", long = "num", default_value = "1")]
    cpu_amount: u32,
}

#[derive(Debug, Fail)]
enum PotCpuError {
    #[fail(display = "no stdout from {}", command)]
    NoStdout { command: String },
    #[fail(display = "{}'s stdout is malformed", command)]
    StdoutMalformed { command: String },
    #[fail(display = "found not UTF-8 character in the {} output", command)]
    Utf8 { command: String },
    #[fail(display = "command {} failed (no success)", command)]
    NoSuccess { command: String },
    #[fail(display = "failed to spawn {} command", command)]
    Spawn { command: String },
}

type Allocation = Vec<u32>;
type AllocationRef = [u32];

fn allocation_from_utf8(v: &[u8]) -> Result<Allocation, PotCpuError> {
    if let Ok(output_string) = std::str::from_utf8(v) {
        if let Some(first_line) = output_string.lines().nth(0) {
            if let Some(mask) = first_line.split(':').nth(1) {
                let result: Vec<u32> = mask
                    .split(',')
                    .map(str::trim)
                    .map(str::parse)
                    .filter(std::result::Result::is_ok)
                    .map(std::result::Result::unwrap)
                    .collect();
                return Ok(result);
            } else {
                return Err(PotCpuError::StdoutMalformed {
                    command: "cpuset".to_string(),
                });
            }
        } else {
            return Err(PotCpuError::NoStdout {
                command: "cpuset".to_string(),
            });
        }
    } else {
        return Err(PotCpuError::Utf8 {
            command: "cpuset".to_string(),
        });
    }
}

fn allocation_to_string(allocation: &AllocationRef, ncpu: u32) -> String {
    if allocation.len() as u32 == ncpu {
        "not restricted".to_string()
    } else {
        let mut result = String::new();
        allocation.iter().for_each(|x| {
            result.push_str(&x.to_string());
            result.push(' ');
        });
        result
    }
}

fn get_ncpu() -> Result<u32, PotCpuError> {
    let output = PCommand::new("/sbin/sysctl")
        .arg("-n")
        .arg("hw.ncpu")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    if let Ok(output) = output {
        if !output.status.success() {
            Err(PotCpuError::NoSuccess {
                command: "sysctl".to_string(),
            })
        } else if let Ok(output_string) = std::str::from_utf8(&output.stdout) {
            if let Ok(ncpu) = output_string.trim().parse::<u32>() {
                Ok(ncpu)
            } else {
                Err(PotCpuError::StdoutMalformed {
                    command: "sysctl".to_string(),
                })
            }
        } else {
            Err(PotCpuError::Utf8 {
                command: "sysctl".to_string(),
            })
        }
    } else {
        Err(PotCpuError::Spawn {
            command: "sysctl".to_string(),
        })
    }
}

fn get_cpusets(conf: &SystemConf) -> Result<HashMap<String, Allocation>, PotCpuError> {
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
            let allocation = allocation_from_utf8(&output.stdout)?;
            result.insert(pot, allocation);
        } else {
            return Err(PotCpuError::Spawn {
                command: "cpuset".to_string(),
            });
        }
    }
    Ok(result)
}

fn get_potcpuconstraints(
    allocations: &HashMap<String, Allocation>,
) -> Result<HashMap<String, u32>, PotCpuError> {
    let mut result = HashMap::new();
    let ncpu = get_ncpu()?;
    for (pot_name, allocation) in allocations {
        if allocation.len() as u32 == ncpu {
            continue;
        }
        result.insert(pot_name.to_string(), allocation.len() as u32);
    }
    Ok(result)
}

fn show(opt: &Opt, conf: &SystemConf) -> Result<(), Error> {
    let ncpu = get_ncpu()?;
    let pot_cpusets = get_cpusets(conf)?;
    let pot_constraints = get_potcpuconstraints(&pot_cpusets)?;
    for (pot_name, allocation) in pot_cpusets {
        let constraint_string = match pot_constraints.iter().find(|(name, _)| *name == &pot_name) {
            Some(constraint) => constraint.1.to_string(),
            None => "NA".to_string(),
        };
        println!("pot {}:", pot_name);
        println!("\tCPU requested: {}", constraint_string);
        println!("\tCPU used: {}", allocation_to_string(&allocation, ncpu));
    }
    if opt.verbose.get_level_filter() > log::LevelFilter::Warn {
        let cpu_allocations = get_cpu_allocation(conf)?;
        for (cpu, pots) in cpu_allocations
            .into_iter()
            .sorted_by_key(|(cpu, _pots)| *cpu)
        {
            println!("CPU {} : allocated {} pots", cpu, pots);
        }
    }
    Ok(())
}

fn get_cpu_allocation(conf: &SystemConf) -> Result<HashMap<u32, u32>, PotCpuError> {
    let pot_cpusets = get_cpusets(conf)?;
    let ncpu = get_ncpu()?;
    let mut result: HashMap<u32, u32> = HashMap::new();
    for i in 0..ncpu {
        result.insert(i, 0);
    }
    for allocations in pot_cpusets.values() {
        for cpu_num in allocations {
            let old_value = result.remove(cpu_num).unwrap();
            result.insert(*cpu_num, old_value + 1);
        }
    }
    Ok(result)
}

fn get_cpu(_opt: &Opt, conf: &SystemConf, cpu_amount: u32) -> Result<(), Error> {
    let ncpu = get_ncpu()?;
    if ncpu <= cpu_amount {
        info!("Not enough CPU in the system to provide a meaningful allocation");
        return Ok(());
    }
    let cpu_allocations = get_cpu_allocation(conf)?;
    let sorted_cpu_allocations = cpu_allocations
        .iter()
        .sorted_by_key(|(cpu, _allocations)| *cpu)
        .sorted_by_key(|(_cpu, allocations)| *allocations);
    let mut cpu_string = String::new();
    for (cpu, _) in sorted_cpu_allocations.take(cpu_amount as usize) {
        cpu_string.push_str(&cpu.to_string());
        cpu_string.push(',');
    }
    println!("{}", cpu_string.trim_end_matches(','));
    Ok(())
}

fn rebalance(_opt: &Opt, conf: &SystemConf) -> Result<(), Error> {
    let cpu_counters = get_cpu_allocation(conf)?;
    let min = cpu_counters
        .iter()
        .min_by_key(|(_cpu, allocation)| *allocation)
        .unwrap();
    let max = cpu_counters
        .iter()
        .max_by_key(|(_cpu, allocation)| *allocation)
        .unwrap();
    if (max.1 - min.1) < 1 {
        println!("no need to rebalance");
        return Ok(());
    }
    let ncpu = get_ncpu()?;
    let pot_allocations = get_cpusets(conf)?;
    let pot_constraints = get_potcpuconstraints(&pot_allocations)?;
    let mut pot_new_allocations = HashMap::new();
    let mut cpu_index_counter: u32 = 0;
    for (pot_name, amount_cpu) in pot_constraints.iter().sorted_by(|a, b| a.0.cmp(&b.0)) {
        let mut cpus: Vec<u32> = Vec::new();
        for _ in 0..*amount_cpu {
            cpus.push(cpu_index_counter);
            cpu_index_counter += 1;
            cpu_index_counter %= ncpu;
        }
        pot_new_allocations.insert(pot_name, cpus);
    }
    for (pot_name, pot_allocation) in pot_new_allocations {
        let mut cpuset_string = String::new();
        for cpu in pot_allocation {
            cpuset_string.push_str(&cpu.to_string());
            cpuset_string.push(',');
        }
        println!(
            "cpuset -l {} -j {}",
            cpuset_string.trim_end_matches(','),
            pot_name
        );
    }
    Ok(())
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
        Command::Show => show(&opt, &conf)?,
        Command::GetCpu(cmd_opt) => get_cpu(&opt, &conf, cmd_opt.cpu_amount)?,
        Command::Rebalance => rebalance(&opt, &conf)?,
    }
    Ok(())
}
