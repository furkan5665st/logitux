use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DpiSpec {
    Range { min: u16, max: u16, step: u16 },
    Discrete(Vec<u16>),
}

impl DpiSpec {
    pub fn from_2201_params(params: &[u8]) -> Self {
        let mut values = Vec::new();
        let mut i = 1;
        while i + 1 < params.len() {
            let v = u16::from_be_bytes([params[i], params[i + 1]]);
            if v == 0 {
                break;
            }
            values.push(v);
            i += 2;
        }
        if values.len() == 3 && (values[1] & 0xE000) == 0xE000 {
            let step = values[1] & 0x1FFF;
            Self::Range {
                min: values[0],
                max: values[2],
                step: step.max(1),
            }
        } else {
            Self::Discrete(values)
        }
    }

    pub fn clamp(&self, dpi: u16) -> u16 {
        match self {
            Self::Range { min, max, step } => {
                let clamped = dpi.clamp(*min, *max);
                let step = (*step).max(1);
                let offset = ((clamped - *min) + step / 2) / step * step;
                (*min + offset).min(*max)
            }
            Self::Discrete(list) => list
                .iter()
                .copied()
                .min_by_key(|&v| v.abs_diff(dpi))
                .unwrap_or(dpi),
        }
    }

    pub fn min_dpi(&self) -> u16 {
        match self {
            Self::Range { min, .. } => *min,
            Self::Discrete(l) => l.iter().copied().min().unwrap_or(400),
        }
    }

    pub fn max_dpi(&self) -> u16 {
        match self {
            Self::Range { max, .. } => *max,
            Self::Discrete(l) => l.iter().copied().max().unwrap_or(8000),
        }
    }

    pub fn step_dpi(&self) -> u16 {
        match self {
            Self::Range { step, .. } => *step,
            Self::Discrete(_) => 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDpiState {
    pub sensor_index: u8,
    pub current_dpi: u16,
    pub current_dpi_y: Option<u16>,
    pub default_dpi: u16,
    pub spec: DpiSpec,
    pub lod: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_g102_dpi_range_decode() {
        let raw = [0x00, 0x00, 0xC8, 0xE0, 0x32, 0x1F, 0x40, 0x00, 0x00];
        let spec = DpiSpec::from_2201_params(&raw);
        match spec {
            DpiSpec::Range { min, max, step } => {
                assert_eq!((min, max, step), (200, 8000, 50));
            }
            _ => panic!("Expected Range"),
        }
        assert_eq!(spec.clamp(823), 800);
        assert_eq!(spec.clamp(830), 850);
        assert_eq!(spec.clamp(100), 200);
        assert_eq!(spec.clamp(25000), 8000);
    }
}
