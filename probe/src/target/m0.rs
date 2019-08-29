use crate::target::BasicRegisterAddresses;
use crate::flash_writer::FlashAlgorithm;
use crate::debug_probe::{
    MasterProbe,
    CpuInformation,
    DebugProbeError,
};
use memory::MI;
use super::{
    TargetRegister,
    CoreRegisterAddress,
    Target,
};
use bitfield::bitfield;

bitfield!{
    #[derive(Copy, Clone)]
    pub struct Dhcsr(u32);
    impl Debug;
    pub s_reset_st, _: 25;
    pub s_retire_st, _: 24;
    pub s_lockup, _: 19;
    pub s_sleep, _: 18;
    pub s_halt, _: 17;
    pub s_regrdy, _: 16;
    pub _, set_c_maskints: 3;
    pub _, set_c_step: 2;
    pub _, set_c_halt: 1;
    pub _, set_c_debugen: 0;
}

impl Dhcsr {
    /// This function sets the bit to enable writes to this register.
    /// 
    /// C1.6.3 Debug Halting Control and Status Register, DHCSR:
    /// Debug key:
    /// Software must write 0xA05F to this field to enable write accesses to bits
    /// [15:0], otherwise the processor ignores the write access.
    pub fn enable_write(&mut self) {
        self.0 |= 0xa05f << 16;
    }
}

impl From<u32> for Dhcsr {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Dhcsr> for u32 {
    fn from(value: Dhcsr) -> Self {
        value.0
    }
}

impl TargetRegister for Dhcsr {
    const ADDRESS: u32 = 0xE000_EDF0;
    const NAME: &'static str = "DHCSR";
}

bitfield!{
    #[derive(Copy, Clone)]
    pub struct Dcrsr(u32);
    impl Debug;
    pub _, set_regwnr: 16;
    pub _, set_regsel: 4,0;
}

impl From<u32> for Dcrsr {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Dcrsr> for u32 {
    fn from(value: Dcrsr) -> Self {
        value.0
    }
}

impl TargetRegister for Dcrsr {
    const ADDRESS: u32 = 0xE000_EDF4;
    const NAME: &'static str = "DCRSR";
}

#[derive(Debug, Copy, Clone)]
pub struct Dcrdr(u32);

impl From<u32> for Dcrdr {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Dcrdr> for u32 {
    fn from(value: Dcrdr) -> Self {
        value.0
    }
}

impl TargetRegister for Dcrdr {
    const ADDRESS: u32 = 0xE000_EDF8;
    const NAME: &'static str = "DCRDR";
}

bitfield!{
    #[derive(Copy, Clone)]
    pub struct BpCtrl(u32);
    impl Debug;
    /// The number of breakpoint comparators. If NUM_CODE is zero, the implementation does not support any comparators
    pub numcode, _: 7, 4;
    /// RAZ on reads, SBO, for writes. If written as zero, the write to the register is ignored.
    pub key, set_key: 1;
    /// Enables the BPU:
    /// 0 BPU is disabled.
    /// 1 BPU is enabled.
    /// This bit is set to 0 on a power-on reset
    pub _, set_enable: 0;
}

impl From<u32> for BpCtrl {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<BpCtrl> for u32 {
    fn from(value: BpCtrl) -> Self {
        value.0
    }
}

impl TargetRegister for BpCtrl {
    const ADDRESS: u32 = 0xE000_2000;
    const NAME: &'static str = "BP_CTRL";
}

bitfield!{
    #[derive(Copy, Clone)]
    pub struct BpCompx(u32);
    impl Debug;
    /// BP_MATCH defines the behavior when the COMP address is matched:
    /// - 00 no breakpoint matching.
    /// - 01 breakpoint on lower halfword, upper is unaffected.
    /// - 10 breakpoint on upper halfword, lower is unaffected.
    /// - 11 breakpoint on both lower and upper halfwords.
    /// - The field is UNKNOWN on reset.
    pub _, set_bp_match: 31,30;
    /// Stores bits [28:2] of the comparison address. The comparison address is
    /// compared with the address from the Code memory region. Bits [31:29] and
    /// [1:0] of the comparison address are zero.
    /// The field is UNKNOWN on power-on reset.
    pub _, set_comp: 28,2;
    /// Enables the comparator:
    /// 0 comparator is disabled.
    /// 1 comparator is enabled.
    /// This bit is set to 0 on a power-on reset.
    pub _, set_enable: 0;
}

impl From<u32> for BpCompx {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<BpCompx> for u32 {
    fn from(value: BpCompx) -> Self {
        value.0
    }
}

impl TargetRegister for BpCompx {
    const ADDRESS: u32 = 0xE000_2008;
    const NAME: &'static str = "BP_CTRL0";
}

pub const R0: CoreRegisterAddress = CoreRegisterAddress(0b00000);
pub const R1: CoreRegisterAddress = CoreRegisterAddress(0b00001);
pub const R2: CoreRegisterAddress = CoreRegisterAddress(0b00010);
pub const R3: CoreRegisterAddress = CoreRegisterAddress(0b00011);
pub const R4: CoreRegisterAddress = CoreRegisterAddress(0b00100);
pub const R9: CoreRegisterAddress = CoreRegisterAddress(0b01001);
pub const SP: CoreRegisterAddress = CoreRegisterAddress(0b01101);
pub const LR: CoreRegisterAddress = CoreRegisterAddress(0b01110);
pub const PC: CoreRegisterAddress = CoreRegisterAddress(0b01111);

pub struct M0;

impl M0 {
    fn wait_for_core_register_transfer(&self, mi: &mut impl MI) -> Result<(), DebugProbeError> {
        // now we have to poll the dhcsr register, until the dhcsr.s_regrdy bit is set
        // (see C1-292, cortex m0 arm)
        for _ in 0..100 {
            let dhcsr_val = Dhcsr(mi.read32(Dhcsr::ADDRESS)?);

            if dhcsr_val.s_regrdy() {
                return Ok(());
            }
        }
        Err(DebugProbeError::Timeout)
    }
}

impl Target for M0 {
    fn get_flash_algorithm(&self) -> FlashAlgorithm {
        FlashAlgorithm {
            load_address: 0x20000000,
            instructions: &[
                0xE00ABE00, 0x062D780D, 0x24084068, 0xD3000040, 0x1E644058, 0x1C49D1FA, 0x2A001E52, 0x4770D1F2,
                0x03004601, 0x28200e00, 0x0940d302, 0xe0051d00, 0xd3022810, 0x1cc00900, 0x0880e000, 0xd50102c9,
                0x43082110, 0x48424770, 0x60414940, 0x60414941, 0x60012100, 0x22f068c1, 0x60c14311, 0x06806940,
                0x483ed406, 0x6001493c, 0x60412106, 0x6081493c, 0x47702000, 0x69014836, 0x43110542, 0x20006101,
                0xb5104770, 0x69014832, 0x43212404, 0x69016101, 0x431103a2, 0x49336101, 0xe0004a30, 0x68c36011,
                0xd4fb03db, 0x43a16901, 0x20006101, 0xb530bd10, 0xffb6f7ff, 0x68ca4926, 0x431a23f0, 0x240260ca,
                0x690a610c, 0x0e0006c0, 0x610a4302, 0x03e26908, 0x61084310, 0x4a214823, 0x6010e000, 0x03ed68cd,
                0x6908d4fb, 0x610843a0, 0x060068c8, 0xd0030f00, 0x431868c8, 0x200160c8, 0xb570bd30, 0x1cc94d14,
                0x68eb0889, 0x26f00089, 0x60eb4333, 0x612b2300, 0xe0174b15, 0x431c692c, 0x6814612c, 0x68ec6004,
                0xd4fc03e4, 0x0864692c, 0x612c0064, 0x062468ec, 0xd0040f24, 0x433068e8, 0x200160e8, 0x1d00bd70,
                0x1f091d12, 0xd1e52900, 0xbd702000, 0x45670123, 0x40023c00, 0xcdef89ab, 0x00005555, 0x40003000,
                0x00000fff, 0x0000aaaa, 0x00000201, 0x00000000
            ],
            pc_init: Some(0x20000047),
            pc_uninit: Some(0x20000075),
            pc_program_page: 0x200000fb,
            pc_erase_sector: 0x200000af,
            pc_erase_all: Some(0x20000083),
            static_base: 0x20000000 + 0x00000020 + 0x0000014c,
            begin_stack: 0x20000000 + 0x00000800,
            begin_data: 0x20002000,
            page_buffers: &[0x20003000, 0x20004000],
            min_program_length: 2,
            analyzer_supported: true,
            analyzer_address: 0x20002000,
        }
    }

    fn get_basic_register_addresses(&self) -> BasicRegisterAddresses {
        BasicRegisterAddresses {
            R0, R1, R2, R3, R9, PC, LR, SP,
        }
    }

    fn wait_for_core_halted(&self, mi: &mut MasterProbe) -> Result<(), DebugProbeError> {
        // Wait until halted state is active again.
        for _ in 0..100 {
            let dhcsr_val = Dhcsr(mi.read32(Dhcsr::ADDRESS)?);

            if dhcsr_val.s_halt() {
                return Ok(());
            }
        }
        Err(DebugProbeError::Timeout)
    }

    fn read_core_reg(&self, mi: &mut MasterProbe, addr: CoreRegisterAddress) -> Result<u32, DebugProbeError> {
        // Write the DCRSR value to select the register we want to read.
        let mut dcrsr_val = Dcrsr(0);
        dcrsr_val.set_regwnr(false); // Perform a read.
        dcrsr_val.set_regsel(addr.into());  // The address of the register to read.

        mi.write32(Dcrsr::ADDRESS, dcrsr_val.into())?;

        self.wait_for_core_register_transfer(mi)?;

        mi.read32(Dcrdr::ADDRESS).map_err(From::from)
    }

    fn write_core_reg(&self, mi: &mut MasterProbe, addr: CoreRegisterAddress, value: u32) -> Result<(), DebugProbeError> {
        // write the DCRSR value to select the register we want to write.
        let mut dcrsr_val = Dcrsr(0);
        dcrsr_val.set_regwnr(true); // Perform a write.
        dcrsr_val.set_regsel(addr.into()); // The address of the register to write.

        mi.write32(Dcrsr::ADDRESS, dcrsr_val.into())?;

        self.wait_for_core_register_transfer(mi)?;

        let result: Result<(), DebugProbeError> = mi.write32(Dcrdr::ADDRESS, value).map_err(From::from);
        result?;

        self.wait_for_core_register_transfer(mi)
    }

    fn halt(&self, mi: &mut MasterProbe) -> Result<CpuInformation, DebugProbeError> {
        // TODO: Generic halt support

        let mut value = Dhcsr(0);
        value.set_c_halt(true);
        value.set_c_debugen(true);
        value.enable_write();

        mi.write32(Dhcsr::ADDRESS, value.into())?;

        // try to read the program counter
        let pc_value = self.read_core_reg(mi, PC)?;

        // get pc
        Ok(CpuInformation {
            pc: pc_value,
        })
    }

    fn run(&self, mi: &mut MasterProbe) -> Result<(), DebugProbeError> {
        let mut value = Dhcsr(0);
        value.set_c_halt(false);
        value.set_c_debugen(false);
        value.enable_write();

        mi.write32(Dhcsr::ADDRESS, value.into()).map_err(Into::into)
    }

    fn step(&self, mi: &mut MasterProbe) -> Result<CpuInformation, DebugProbeError> {
        let mut value = Dhcsr(0);
        // Leave halted state.
        // Step one instruction.
        value.set_c_step(true);
        value.set_c_halt(false);
        value.set_c_debugen(true);
        value.set_c_maskints(true);
        value.enable_write();

        mi.write32(Dhcsr::ADDRESS, value.into())?;

        self.wait_for_core_halted(mi)?;

        // try to read the program counter
        let pc_value = self.read_core_reg(mi, PC)?;

        // get pc
        Ok(CpuInformation {
            pc: pc_value,
        })
    }

    fn get_available_breakpoint_units(&self, mi: &mut MasterProbe) -> Result<u32, DebugProbeError> {
        let result = mi.read32(BpCtrl::ADDRESS)?;

        self.wait_for_core_register_transfer(mi)?;

        Ok(result)
    }

    fn enable_breakpoints(&self, mi: &mut MasterProbe, state: bool) -> Result<(), DebugProbeError> {
        let mut value = BpCtrl(0);
        value.set_enable(state);

        mi.write32(BpCtrl::ADDRESS, value.into())?;

        self.wait_for_core_halted(mi)
    }

    fn set_breakpoint(&self, mi: &mut MasterProbe, addr: u32) -> Result<(), DebugProbeError> {
        let mut value = BpCompx(0);
        value.set_bp_match(0b11);
        value.set_comp((addr >> 2) | 0x00FFFFFF);
        value.set_enable(true);

        mi.write32(BpCtrl::ADDRESS, value.into())?;

        self.wait_for_core_halted(mi)
    }

    fn enable_breakpoint(&self, _mi: &mut MasterProbe, _addr: u32) -> Result<(), DebugProbeError> {
        unimplemented!();
    }

    fn disable_breakpoint(&self, _mi: &mut MasterProbe, _addr: u32) -> Result<(), DebugProbeError> {
        unimplemented!();
    }
}