use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::input_synth;
use crate::integrations;
use crate::store::{DiscordSettings, MacroKind, MacroStep, ObsSettings, UserMacro};

struct ActiveLoop {
    stop_flag: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct MacroRuntime {
    held_flags: Mutex<HashMap<usize, Arc<AtomicBool>>>,
    toggled_loops: Mutex<HashMap<String, ActiveLoop>>,
}

impl MacroRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_toggle_running(&self, macro_id: &str) -> bool {
        let Ok(guard) = self.toggled_loops.lock() else {
            return false;
        };
        guard
            .get(macro_id)
            .is_some_and(|l| !l.stop_flag.load(Ordering::Relaxed))
    }

    pub fn stop_all(&self) {
        if let Ok(mut h) = self.held_flags.lock() {
            for f in h.values() {
                f.store(false, Ordering::Relaxed);
            }
            h.clear();
        }
        if let Ok(mut t) = self.toggled_loops.lock() {
            for l in t.values() {
                l.stop_flag.store(true, Ordering::Relaxed);
            }
            t.clear();
        }
    }

    pub fn on_button_press(
        &self,
        button_idx: usize,
        mac: UserMacro,
        discord_cfg: DiscordSettings,
        obs_cfg: ObsSettings,
    ) {
        let fallback_delay = if mac.use_standard_delay {
            mac.standard_delay_ms.max(5)
        } else {
            15
        };

        match mac.kind {
            MacroKind::NoRepeat => {
                let steps = mac.steps.clone();
                thread::spawn(move || {
                    run_step_list(&steps, fallback_delay, &discord_cfg, &obs_cfg, None);
                });
            }
            MacroKind::RepeatWhileHolding => {
                let held = Arc::new(AtomicBool::new(true));
                if let Ok(mut map) = self.held_flags.lock() {
                    if let Some(prev) = map.insert(button_idx, held.clone()) {
                        prev.store(false, Ordering::Relaxed);
                    }
                }
                let steps = mac.steps.clone();
                thread::spawn(move || {
                    while held.load(Ordering::Relaxed) {
                        run_step_list(&steps, fallback_delay, &discord_cfg, &obs_cfg, Some(&held));
                        thread::sleep(Duration::from_millis(fallback_delay as u64));
                    }
                });
            }
            MacroKind::Toggle => {
                if let Ok(mut map) = self.toggled_loops.lock() {
                    if let Some(existing) = map.remove(&mac.id) {
                        if !existing.stop_flag.load(Ordering::Relaxed) {
                            existing.stop_flag.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                    let stop = Arc::new(AtomicBool::new(false));
                    map.insert(
                        mac.id.clone(),
                        ActiveLoop {
                            stop_flag: stop.clone(),
                        },
                    );
                    let steps = mac.steps.clone();
                    thread::spawn(move || {
                        while !stop.load(Ordering::Relaxed) {
                            run_step_list(&steps, fallback_delay, &discord_cfg, &obs_cfg, None);
                            thread::sleep(Duration::from_millis(fallback_delay as u64));
                        }
                    });
                }
            }
            MacroKind::Sequence => {
                let held = Arc::new(AtomicBool::new(true));
                if let Ok(mut map) = self.held_flags.lock() {
                    if let Some(prev) = map.insert(button_idx, held.clone()) {
                        prev.store(false, Ordering::Relaxed);
                    }
                }
                let on_press = mac.on_press.clone();
                let while_holding = mac.while_holding.clone();
                let on_release = mac.on_release.clone();
                thread::spawn(move || {
                    run_step_list(&on_press, fallback_delay, &discord_cfg, &obs_cfg, None);
                    while held.load(Ordering::Relaxed) {
                        if while_holding.is_empty() {
                            thread::sleep(Duration::from_millis(12));
                        } else {
                            run_step_list(
                                &while_holding,
                                fallback_delay,
                                &discord_cfg,
                                &obs_cfg,
                                Some(&held),
                            );
                            thread::sleep(Duration::from_millis(fallback_delay as u64));
                        }
                    }
                    run_step_list(&on_release, fallback_delay, &discord_cfg, &obs_cfg, None);
                });
            }
        }
    }

    pub fn on_button_release(&self, button_idx: usize) {
        if let Ok(mut map) = self.held_flags.lock() {
            if let Some(flag) = map.remove(&button_idx) {
                flag.store(false, Ordering::Relaxed);
            }
        }
    }
}

fn run_step_list(
    steps: &[MacroStep],
    default_delay_ms: u32,
    discord_cfg: &DiscordSettings,
    obs_cfg: &ObsSettings,
    keep_running: Option<&AtomicBool>,
) {
    for step in steps {
        if let Some(flag) = keep_running {
            if !flag.load(Ordering::Relaxed) {
                break;
            }
        }
        match step {
            MacroStep::KeyDown { modifiers, hid_key } => {
                input_synth::key_down(*modifiers, *hid_key);
                if default_delay_ms > 0 {
                    thread::sleep(Duration::from_millis(default_delay_ms as u64));
                }
            }
            MacroStep::KeyUp { modifiers, hid_key } => {
                input_synth::key_up(*modifiers, *hid_key);
                if default_delay_ms > 0 {
                    thread::sleep(Duration::from_millis(default_delay_ms as u64));
                }
            }
            MacroStep::KeyTap { modifiers, hid_key } => {
                input_synth::tap_shortcut(*modifiers, *hid_key);
                if default_delay_ms > 0 {
                    thread::sleep(Duration::from_millis(default_delay_ms as u64));
                }
            }
            MacroStep::MouseClick { button } => {
                input_synth::mouse_click(*button);
                if default_delay_ms > 0 {
                    thread::sleep(Duration::from_millis(default_delay_ms as u64));
                }
            }
            MacroStep::DelayMs(ms) => {
                thread::sleep(Duration::from_millis((*ms).max(1) as u64));
            }
            MacroStep::TypeText(txt) => {
                input_synth::type_text(txt);
            }
            MacroStep::Media(m) => {
                input_synth::trigger_media(*m);
            }
            MacroStep::Action(act) => {
                let _ = integrations::execute_integration(act, discord_cfg, obs_cfg);
            }
        }
    }
}
