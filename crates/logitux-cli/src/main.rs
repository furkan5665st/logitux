use clap::{Parser, Subcommand};
use logitux_hidpp::features::{
    onboard::OnboardMode,
    perkey::ZoneColor,
    rgb::{RgbColor, RgbEffect, WaveDirection},
};
use logitux_transport::{connect_all_devices, enumerate_logitech_devices};

#[derive(Parser)]
#[command(
    name = "logitux",
    version,
    about = "LogiTux command line tool"
)]
struct Cli {
    /// Emit structured JSON output
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List raw Logitech HID++ interfaces visible to the OS
    List,
    /// Print device information and feature table
    Probe,
    /// Get or set optical sensor DPI
    Dpi {
        /// New DPI value (e.g. 800, 1600, 3200)
        value: Option<u16>,
        /// Sensor index (default 0)
        #[arg(long, default_value_t = 0)]
        sensor: u8,
    },
    /// Get or set USB polling rate in Hz (125, 250, 500, 1000, ...)
    Rate {
        /// New polling rate in Hz
        hz: Option<u16>,
    },
    /// Control RGB lighting effects (0x8071 / 0x8070)
    Rgb {
        /// Mode: off, fixed, breathing, cycle, wave, blend
        mode: String,
        /// Hex color for fixed/breathing (e.g. 00B8FC)
        #[arg(long, default_value = "00B8FC")]
        color: String,
        /// Animation period in ms
        #[arg(long, default_value_t = 3000)]
        period: u16,
        /// Brightness 0..100
        #[arg(long, default_value_t = 100)]
        brightness: u8,
        /// Cluster/zone index
        #[arg(long, default_value_t = 0)]
        cluster: u8,
        /// Save persistently to device NV memory
        #[arg(long)]
        persist: bool,
    },
    /// Set per-zone RGB colors (0x8081)
    Perkey {
        /// Three hex colors for Left, Middle, Right zones (e.g. FF0055 00B8FC 7700FF)
        #[arg(required = true)]
        colors: Vec<String>,
        /// Starting LED zone ID (default 1: G102 Left=1, Center=2, Right=3)
        #[arg(long, default_value_t = 1)]
        start_id: u8,
        /// Persist frame
        #[arg(long)]
        persist: bool,
    },
    /// Inspect or switch Onboard Memory mode & profiles (Feature 0x8100)
    Onboard {
        /// Optional action: info, dump, host, onboard
        #[arg(default_value = "info")]
        action: String,
    },
    /// Send a raw HID++ 2.0 command: <feature_hex> <fn_id> [hex_bytes...]
    Raw {
        feature_hex: String,
        fn_id: u8,
        params: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::List => {
            let devs = enumerate_logitech_devices()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&devs)?);
            } else if devs.is_empty() {
                println!("No Logitech HID++ devices found.");
            } else {
                for (i, d) in devs.iter().enumerate() {
                    let meta = logitux_devices::lookup_by_pid(d.product_id)
                        .map(|m| m.model_name)
                        .unwrap_or("Generic Logitech HID++");
                    println!(
                        "[{i}] {:04X}:{:04X} {} ({}) | iface={} receiver={}",
                        d.vendor_id,
                        d.product_id,
                        d.product_name,
                        meta,
                        d.interface_number,
                        d.is_receiver
                    );
                    if let Some(p) = &d.short_path {
                        println!("    short: {p}");
                    }
                    if let Some(p) = &d.long_path {
                        println!("    long : {p}");
                    }
                }
            }
        }
        Commands::Probe => {
            let connected = connect_all_devices()?;
            if connected.is_empty() {
                println!("No active Logitech HID++ 2.0 devices responded.");
                return Ok(());
            }
            let summaries: Vec<_> = connected.iter().map(|c| c.device.probe_summary()).collect();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&summaries)?);
            } else {
                for (c, s) in connected.iter().zip(summaries.iter()) {
                    println!("============================================================");
                    println!(
                        "Device: {} (VID:0x{:04X} PID:0x{:04X} devIdx:0x{:02X})",
                        s.name, c.descriptor.vendor_id, c.descriptor.product_id, s.device_index
                    );
                    println!(
                        "HID++ Protocol: {}.{} | Kind: {:?}",
                        s.protocol_version.0, s.protocol_version.1, s.kind
                    );
                    if let Some(fw) = &s.fw_info {
                        println!(
                            "Unit ID: {} | Model ID: {} (ext={})",
                            fw.unit_id_hex(),
                            fw.model_id_hex(),
                            fw.extended_model_id
                        );
                        for e in &fw.entities {
                            println!(
                                "  FW[{}] {:16} {} v{} build 0x{:04X} active={} pid=0x{:04X}",
                                e.index,
                                e.type_name(),
                                e.prefix,
                                e.version,
                                e.build,
                                e.active,
                                e.transport_pid
                            );
                        }
                    }
                    println!("\nHID++ 2.0 Feature Table ({} features):", s.features.len());
                    for f in &s.features {
                        let lock = if f.is_safety_locked() {
                            " [SAFETY-LOCKED]"
                        } else {
                            ""
                        };
                        println!(
                            "  [{:02}] 0x{:04X} {:32} v{} flags=0x{:02X}{lock}",
                            f.index,
                            f.id,
                            f.name(),
                            f.version,
                            f.flags
                        );
                    }
                    if let Some(dpis) = &s.dpi {
                        println!("\nOptical Sensor DPI:");
                        for d in dpis {
                            println!(
                                "  Sensor {}: current={} DPI (default={} DPI) spec={:?}",
                                d.sensor_index, d.current_dpi, d.default_dpi, d.spec
                            );
                        }
                    }
                    if let Some(rate) = &s.report_rate {
                        println!(
                            "\nPolling Rate: {} Hz (supported: {:?})",
                            rate.current_hz, rate.supported_hz
                        );
                    }
                    if let Some(rgb) = &s.rgb {
                        println!(
                            "\nRGB Lighting (0x{:04X}): {} clusters, flags=0x{:02X}",
                            rgb.feature_id, rgb.cluster_count, rgb.extended_flags
                        );
                        for cl in &rgb.clusters {
                            println!(
                                "  Cluster {} ({} / loc=0x{:04X}): {} effects",
                                cl.cluster_index,
                                cl.location_name,
                                cl.location,
                                cl.effects.len()
                            );
                            for ef in &cl.effects {
                                println!(
                                    "    [{}] 0x{:04X} {:14} caps=0x{:04X} default_period={}ms",
                                    ef.effect_index,
                                    ef.effect_id,
                                    ef.effect_name,
                                    ef.capabilities,
                                    ef.default_period_ms
                                );
                            }
                        }
                    }
                    if let Some(pk) = &s.perkey {
                        println!(
                            "\nPer-Key / Multi-Zone RGB (0x8081): flags=0x{:02X} led_count={} raw={:02X?}",
                            pk.flags, pk.led_count, pk.raw
                        );
                    }
                    if let Some(ob) = &s.onboard {
                        println!(
                            "\nOnboard Profiles (0x8100): mode={:?} profiles={} buttons={} sectors={}x{}B fmt=(mem={}, prof={}, macro={})",
                            s.onboard_mode,
                            ob.profile_count,
                            ob.button_count,
                            ob.sector_count,
                            ob.sector_size,
                            ob.memory_model_id,
                            ob.profile_format_id,
                            ob.macro_format_id
                        );
                    }
                    if let Some(bat) = &s.battery {
                        println!("\nBattery: {:?}", bat);
                    }
                    if let Some(btns) = &s.buttons {
                        println!("\nControls/Buttons ({}):", btns.len());
                        for b in btns {
                            println!(
                                "  [{}] CID=0x{:04X} ({}) task=0x{:04X} flags=0x{:04X} diverted={} remap=0x{:04X}",
                                b.index, b.cid, b.name, b.task_id, b.flags, b.diverted, b.remapped_cid
                            );
                        }
                    }
                    println!("============================================================");
                }
            }
        }
        Commands::Dpi { value, sensor } => {
            let connected = connect_all_devices()?;
            let c = connected.first().ok_or("No Logitech mouse connected")?;
            match value {
                Some(dpi) => {
                    c.device.set_dpi(sensor, dpi)?;
                    println!("DPI set to {dpi} on sensor {sensor}");
                }
                None => {
                    let states = c.device.get_dpi_states()?;
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&states)?);
                    } else {
                        for s in states {
                            println!(
                                "Sensor {}: {} DPI (default {}, range {}..={} step {})",
                                s.sensor_index,
                                s.current_dpi,
                                s.default_dpi,
                                s.spec.min_dpi(),
                                s.spec.max_dpi(),
                                s.spec.step_dpi()
                            );
                        }
                    }
                }
            }
        }
        Commands::Rate { hz } => {
            let connected = connect_all_devices()?;
            let c = connected.first().ok_or("No Logitech mouse connected")?;
            match hz {
                Some(val) => {
                    c.device.set_report_rate(val)?;
                    println!("Polling rate set to {val} Hz");
                }
                None => {
                    let info = c.device.get_report_rate()?;
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&info)?);
                    } else {
                        println!(
                            "Current polling rate: {} Hz (supported: {:?})",
                            info.current_hz, info.supported_hz
                        );
                    }
                }
            }
        }
        Commands::Rgb {
            mode,
            color,
            period,
            brightness,
            cluster,
            persist,
        } => {
            let connected = connect_all_devices()?;
            let c = connected.first().ok_or("No Logitech mouse connected")?;
            let col = RgbColor::from_hex(&color).ok_or("Invalid hex color (use e.g. 00B8FC)")?;
            let eff = match mode.to_lowercase().as_str() {
                "off" => RgbEffect::Off,
                "fixed" | "solid" => RgbEffect::Fixed {
                    color: col,
                    brightness,
                },
                "breathing" | "breathe" => RgbEffect::Breathing {
                    color: col,
                    period_ms: period,
                    brightness,
                },
                "cycle" => RgbEffect::ColorCycle {
                    period_ms: period,
                    brightness,
                },
                "wave" => RgbEffect::ColorWave {
                    period_ms: period,
                    direction: WaveDirection::Horizontal,
                    brightness,
                },
                "blend" => RgbEffect::ColorBlend {
                    period_ms: period,
                    brightness,
                },
                other => return Err(format!("Unknown RGB mode: {other}").into()),
            };
            c.device.set_rgb_sw_control(true)?;
            c.device.set_rgb_effect(cluster, eff, persist)?;
            println!("Applied RGB effect {:?} to cluster {cluster}", eff);
        }
        Commands::Perkey {
            colors,
            start_id,
            persist,
        } => {
            let connected = connect_all_devices()?;
            let c = connected.first().ok_or("No Logitech mouse connected")?;
            let mut zones = Vec::new();
            for (i, hex) in colors.iter().enumerate() {
                let col =
                    RgbColor::from_hex(hex).ok_or_else(|| format!("Invalid hex color: {hex}"))?;
                zones.push(ZoneColor {
                    zone_id: start_id + (i as u8),
                    color: col,
                });
            }
            c.device.set_rgb_sw_control(true).ok();
            c.device.set_perkey_zones(&zones, persist)?;
            println!("Applied {} per-key/zone colors", zones.len());
        }
        Commands::Onboard { action } => {
            let connected = connect_all_devices()?;
            let c = connected.first().ok_or("No Logitech mouse connected")?;
            match action.as_str() {
                "info" => {
                    let desc = c.device.get_onboard_description()?;
                    let mode = c.device.get_onboard_mode()?;
                    let active = c.device.get_active_onboard_profile().ok();
                    let dpi_idx = c.device.get_active_onboard_dpi_index().ok();
                    let dir = c.device.read_onboard_directory().ok();
                    println!("Onboard Description: {desc:#?}");
                    println!("Mode: {mode:?} | Active Profile Sector: {active:?} | Active DPI Index: {dpi_idx:?}");
                    if let Some(entries) = dir {
                        println!("Profile Directory ({}):", entries.len());
                        for e in &entries {
                            println!("  sector=0x{:04X} enabled={}", e.sector_id, e.enabled);
                            if let Ok(prof) = c.device.read_onboard_profile(e.sector_id) {
                                println!(
                                    "    name={:?} rate={}Hz default_dpi_idx={} stages={:?} crc_valid={}",
                                    prof.name,
                                    if prof.report_rate_ms > 0 { 1000 / (prof.report_rate_ms as u16) } else { 0 },
                                    prof.default_dpi_index,
                                    prof.dpi_stages,
                                    prof.crc_valid
                                );
                                println!("    active_rgb_0={:?}", prof.active_effect_0);
                                for (bi, act) in prof.buttons.iter().enumerate() {
                                    println!("    btn[{}]: {}", bi + 1, act.label());
                                }
                            }
                        }
                    }
                }
                "dump" => {
                    let desc = c.device.get_onboard_description()?;
                    for sec in 0..desc.profile_count.min(4) {
                        let data = c.device.read_onboard_sector(sec as u16, desc.sector_size)?;
                        println!("--- Sector 0x{sec:04X} ({} bytes) ---", data.len());
                        for (row, chunk) in data.chunks(16).enumerate() {
                            let hex: Vec<String> =
                                chunk.iter().map(|b| format!("{b:02X}")).collect();
                            println!("  {:04X}: {}", row * 16, hex.join(" "));
                        }
                    }
                }
                "host" => {
                    c.device.set_onboard_mode(OnboardMode::Host)?;
                    println!("Switched mouse to Host (Software) mode");
                }
                "onboard" => {
                    c.device.set_onboard_mode(OnboardMode::Onboard)?;
                    println!("Switched mouse to Onboard Memory mode");
                }
                other => return Err(format!("Unknown onboard action: {other}").into()),
            }
        }
        Commands::Raw {
            feature_hex,
            fn_id,
            params,
        } => {
            let connected = connect_all_devices()?;
            let c = connected.first().ok_or("No Logitech mouse connected")?;
            let fid = u16::from_str_radix(feature_hex.trim_start_matches("0x"), 16)?;
            let mut pbytes = Vec::new();
            for p in params {
                pbytes.push(u8::from_str_radix(p.trim_start_matches("0x"), 16)?);
            }
            let reply = c.device.request_feature(fid, fn_id, &pbytes)?;
            println!("Reply: {}", reply.to_hex());
        }
    }
    Ok(())
}
