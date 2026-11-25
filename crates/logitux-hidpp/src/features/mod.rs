pub mod battery;
pub mod buttons;
pub mod dpi;
pub mod info;
pub mod onboard;
pub mod perkey;
pub mod rate;
pub mod rgb;
pub mod scroll;

use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
pub mod FeatureId {
    pub const IROOT: u16 = 0x0000;
    pub const IFEATURE_SET: u16 = 0x0001;
    pub const IFEATURE_INFO: u16 = 0x0002;
    pub const DEVICE_FW_INFO: u16 = 0x0003;
    pub const DEVICE_UNIT_ID: u16 = 0x0004;
    pub const DEVICE_NAME: u16 = 0x0005;
    pub const DEVICE_GROUPS: u16 = 0x0006;
    pub const DEVICE_FRIENDLY_NAME: u16 = 0x0007;
    pub const KEEP_ALIVE: u16 = 0x0008;
    pub const SUBDEVICES: u16 = 0x0009;
    pub const CONFIG_CHANGE: u16 = 0x0020;
    pub const CRYPTO_ID: u16 = 0x0021;
    pub const TARGET_SOFTWARE: u16 = 0x0030;
    pub const WIRELESS_SIGNAL_STRENGTH: u16 = 0x0080;
    pub const DFU_CONTROL: u16 = 0x00C1;
    pub const DFU_CONTROL_SIGNED: u16 = 0x00C2;
    pub const DFU_CONTROL_2: u16 = 0x00C3;
    pub const DFU: u16 = 0x00D0;
    pub const RESUMABLE_DFU: u16 = 0x00D1;

    pub const BATTERY_STATUS: u16 = 0x1000;
    pub const BATTERY_VOLTAGE: u16 = 0x1001;
    pub const UNIFIED_BATTERY: u16 = 0x1004;
    pub const CHARGING_CONTROL: u16 = 0x1010;
    pub const MONO_LIGHTING: u16 = 0x1300;
    pub const FORCE_PAIRING: u16 = 0x1500;
    pub const CHANGE_HOST: u16 = 0x1814;
    pub const HOSTS_INFO: u16 = 0x1815;
    pub const BLE_PRO_PREPAIRING: u16 = 0x1816;
    pub const ILLUMINATION_LIGHT: u16 = 0x1990;

    pub const SPECIAL_KEYS_MSE_BUTTONS: u16 = 0x1B04;
    pub const FULL_KEY_CUSTOMIZATION: u16 = 0x1B05;
    pub const ANALOG_KEYS: u16 = 0x1B08;
    pub const ANALOG_BUTTONS: u16 = 0x1B0C;
    pub const SWITCH_SWAPABILITY: u16 = 0x1B20;
    pub const DEVICE_MODE: u16 = 0x1B30;
    pub const HID_USAGES: u16 = 0x1BC0;
    pub const WIRELESS_DEVICE_STATUS: u16 = 0x1D4B;
    pub const LEFT_RIGHT_SWAP: u16 = 0x2001;
    pub const SWAP_BUTTONS: u16 = 0x2005;
    pub const VERTICAL_SCROLLING: u16 = 0x2100;
    pub const SMARTSHIFT: u16 = 0x2110;
    pub const SMARTSHIFT_ENHANCED: u16 = 0x2111;
    pub const HIRES_WHEEL: u16 = 0x2120;
    pub const HIRES_WHEEL_V2: u16 = 0x2121;
    pub const LOWRES_WHEEL: u16 = 0x2130;
    pub const THUMBWHEEL: u16 = 0x2150;

    pub const MOUSE_POINTER: u16 = 0x2200;
    pub const ADJUSTABLE_DPI: u16 = 0x2201;
    pub const EXTENDED_ADJUSTABLE_DPI: u16 = 0x2202;
    pub const POINTER_SPEED: u16 = 0x2205;
    pub const ANGLE_SNAPPING: u16 = 0x2230;
    pub const SURFACE_TUNING: u16 = 0x2240;
    pub const HYBRID_TRACKING: u16 = 0x2400;

    pub const GKEYS: u16 = 0x8010;
    pub const MKEYS: u16 = 0x8020;
    pub const MR: u16 = 0x8030;
    pub const BRIGHTNESS_CONTROL: u16 = 0x8040;
    pub const REPORT_RATE: u16 = 0x8060;
    pub const EXTENDED_REPORT_RATE: u16 = 0x8061;
    pub const COLOR_LED_EFFECTS: u16 = 0x8070;
    pub const RGB_EFFECTS: u16 = 0x8071;
    pub const KEY_LIGHTING: u16 = 0x8080;
    pub const PER_KEY_LIGHTING: u16 = 0x8081;
    pub const MODE_STATUS: u16 = 0x8090;
    pub const BUNNY_HOPPING: u16 = 0x80E0;
    pub const ONBOARD_PROFILES: u16 = 0x8100;
    pub const PROFILE_MANAGEMENT: u16 = 0x8101;
    pub const PROFILE_MANAGEMENT_V2: u16 = 0x8102;
    pub const GLOBAL_DEVICE_SETTINGS: u16 = 0x8103;
    pub const MOUSE_BUTTON_SPY: u16 = 0x8110;
    pub const GAMING_ATTACHMENTS: u16 = 0x8120;
}

pub fn feature_name(id: u16) -> &'static str {
    match id {
        0x0000 => "IRoot",
        0x0001 => "IFeatureSet",
        0x0002 => "IFeatureInfo",
        0x0003 => "DeviceFwInfo",
        0x0004 => "DeviceUnitId",
        0x0005 => "DeviceName",
        0x0006 => "DeviceGroups",
        0x0007 => "DeviceFriendlyName",
        0x0008 => "KeepAlive",
        0x0009 => "Subdevices",
        0x0020 => "ConfigChange",
        0x0021 => "CryptoId",
        0x0030 => "TargetSoftware",
        0x0080 => "WirelessSignalStrength",
        0x00C1 => "DfuControl (Safety-Locked)",
        0x00C2 => "DfuControlSigned (Safety-Locked)",
        0x00C3 => "DfuControl2 (Safety-Locked)",
        0x00D0 => "Dfu (Safety-Locked)",
        0x00D1 => "ResumableDfu (Safety-Locked)",
        0x1000 => "BatteryLevelStatus",
        0x1001 => "BatteryVoltage",
        0x1004 => "UnifiedBattery",
        0x1010 => "ChargingControl",
        0x1300 => "MonoLighting",
        0x1500 => "ForcePairing",
        0x1802 => "DeviceReset",
        0x1805 => "OobState",
        0x1806 => "ConfigurableDeviceProperties",
        0x1814 => "ChangeHost",
        0x1815 => "HostsInfo",
        0x1816 => "BleProPrepairing",
        0x1830 => "PowerModes",
        0x1861 => "BatteryLevelsCalibration",
        0x1990 => "IlluminationLight",
        0x1B04 => "SpecialKeysMSEButtons",
        0x1B05 => "FullKeyCustomization",
        0x1B08 => "AnalogKeys",
        0x1B0C => "AnalogButtons",
        0x1B20 => "SwitchSwapability",
        0x1B30 => "DeviceMode",
        0x1BC0 => "HidUsages",
        0x1D4B => "WirelessDeviceStatus",
        0x1DF3 => "EquadDjDebugInfo",
        0x1E00 => "EnableHiddenFeatures",
        0x1E02 => "ManageDeactivatableFeatures",
        0x1E22 => "SpiDirectAccess",
        0x1EB0 => "TdeAccessToNvm",
        0x1F20 => "AdcMeasurement",
        0x2001 => "LeftRightSwap",
        0x2005 => "SwapButtons",
        0x2100 => "VerticalScrolling",
        0x2110 => "SmartShift",
        0x2111 => "SmartShiftEnhanced",
        0x2120 => "HiResWheel",
        0x2121 => "HiResWheelV2",
        0x2130 => "LowResWheel",
        0x2150 => "ThumbWheel",
        0x2200 => "MousePointer",
        0x2201 => "AdjustableDPI",
        0x2202 => "ExtendedAdjustableDPI",
        0x2205 => "PointerSpeed",
        0x2230 => "AngleSnapping",
        0x2240 => "SurfaceTuning",
        0x2400 => "HybridTracking",
        0x40A2 => "FnInversionV2",
        0x40A3 => "FnInversionV3",
        0x4522 => "DisableKeys",
        0x8010 => "GKeys",
        0x8020 => "MKeys",
        0x8030 => "MR",
        0x8040 => "BrightnessControl",
        0x8060 => "ReportRate",
        0x8061 => "ExtendedAdjustableReportRate",
        0x8070 => "ColorLedEffects",
        0x8071 => "RgbEffects",
        0x8080 => "KeyLighting",
        0x8081 => "PerKeyLighting",
        0x8090 => "ModeStatus",
        0x80E0 => "BunnyHopping",
        0x8100 => "OnboardProfiles",
        0x8101 => "ProfileManagement",
        0x8102 => "ProfileManagementV2",
        0x8103 => "GlobalDeviceSettings",
        0x8110 => "MouseButtonSpy",
        0x8120 => "GamingAttachments",
        _ => "UnknownFeature",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEntry {
    pub index: u8,
    pub id: u16,
    pub flags: u8,
    pub version: u8,
}

impl FeatureEntry {
    pub fn name(&self) -> &'static str {
        feature_name(self.id)
    }

    pub fn is_obsolete(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    pub fn is_sw_hidden(&self) -> bool {
        (self.flags & 0x40) != 0
    }

    pub fn is_internal(&self) -> bool {
        (self.flags & 0x20) != 0
    }

    pub fn is_safety_locked(&self) -> bool {
        matches!(
            self.id,
            0x00C1 | 0x00C2 | 0x00C3 | 0x00D0 | 0x00D1 | 0x1E00 | 0x1E22 | 0x1EB0
        )
    }
}
