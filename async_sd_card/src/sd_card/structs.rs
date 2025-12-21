use bitfield::bitfield;
use bitflags::bitflags;

bitfield! {
    pub struct CommandByte0(u8);

    bool; pub get_start_bit, set_start_bit: 7;
    bool; pub get_transmission_bit, set_transmission_bit: 6;
    u8; pub get_command_index, set_command_index: 5, 0;
}

bitfield! {
    pub struct CommandByte5(u8);

    u8; pub get_crc7, set_crc7: 7, 1;
    bool; pub get_end_bit, set_end_bit: 0;
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct R1: u8 {
        const BIT_7 = 1 << 7;
        const PARAMETER_ERROR = 1 << 6;
        const ADDRESS_ERROR = 1 << 5;
        const ERASE_SEQUENCE_ERROR = 1 << 4;
        const COM_CRC_ERROR = 1 << 3;
        const ILLEGAL_COMMAND = 1 << 2;
        const ERASE_RESET = 1 << 1;
        const IN_IDLE_STATE = 1 << 0;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct R2Byte2: u8 {
        const OUT_OF_RANGE_OR_CSD_OVERWRITE = 1 << 7;
        const ERASE_PARAM = 1 << 6;
        const WP_VIOLATION = 1 << 5;
        const CARD_ECC_FAILED = 1 << 4;
        const CC_ERROR = 1 << 3;
        const ERROR = 1 << 2;
        const WP_ERASE_SKIP_OR_LOCK_UNLOCK_CMD_FAILED = 1 << 1;
        const CARD_IS_LOCKED = 1 << 0;
    }
}

bitfield! {
    pub struct R7Byte1(u8);

    u8; pub get_command_version, set_command_version: 7, 4;
}

bitfield! {
    pub struct R7Byte3(u8);

    u8; _get_voltage_accepted, _set_volage_accepted: 3, 0;
}

impl R7Byte3 {
    pub fn get_voltage_accepted(&self) -> VoltageAccpted {
        VoltageAccpted::from_bits_retain(self._get_voltage_accepted())
    }
}

bitfield! {
    pub struct Command8Argument(u32);

    bool; pub get_pcie_1_2v_support, set_pcie1_2v_support: 13;
    bool; pub get_pcie_availability, set_pcie_availability: 12;
    u8; pub get_voltage_accpted, set_voltage_accepted: 11, 8;
    u8; pub get_check_pattern, set_check_pattern: 7, 0;
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct VoltageAccpted: u8 {
        const _2_7V_3_6V = 1 << 0;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Ocr: u32 {
        const _2_7V_2_8V = 1 << 15;
        const _2_8V_2_9V = 1 << 16;
        const _2_9V_3_0V = 1 << 17;
        const _3_0V_3_1V = 1 << 18;
        const _3_1V_3_2V = 1 << 19;
        const _3_2V_3_3V = 1 << 20;
        const _3_3V_3_4V = 1 << 21;
        const _3_4V_3_5V = 1 << 22;
        const _2_5V_3_6V = 1 << 23;
        const S18A = 1 << 24;
        const CO2T = 1 << 27;
        const UHS_II = 1 << 29;
        const CARD_CAPACITY_STATUS = 1 << 30;
        const CARD_POWER_UP_STATUS = 1 << 31;
    }
}

impl Ocr {
    /// If the SD card supports 3.3V, according to its OCR
    pub fn supports_3_3v(&self) -> bool {
        self.contains(Self::_3_2V_3_3V) || self.contains(Self::_3_3V_3_4V)
    }

    pub fn is_powered_up(&self) -> bool {
        self.contains(Self::CARD_POWER_UP_STATUS)
    }

    /// Note that if the card is powered up, the `CCS` bit is not valid, so the card must be powered up to know.
    pub fn supports_sdhc_or_sdxc(&self) -> Option<bool> {
        if self.is_powered_up() {
            Some(self.contains(Self::CARD_CAPACITY_STATUS))
        } else {
            None
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct CommandA41Argument: u32 {
        const HCS = 1 << 30;
    }
}

bitfield! {
    pub struct DataResponseToken(u8);

    bool; pub get_bit_4, set_bit_4: 4;
    u8; pub get_status, set_status: 3, 1;
    bool; pub get_bit_0, set_bit_0: 0;
}

bitfield! {
    pub struct CsdV2(u128);

    u32; pub get_c_size, set_c_size: 75, 48;
}

impl CsdV2 {
    pub fn card_capacity_bytes(&self) -> u64 {
        (u64::from(self.get_c_size()) + 1) * 512 * 1024
    }
}
