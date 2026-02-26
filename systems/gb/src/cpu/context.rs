pub trait CpuContext {
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
    // Advance hardware by DMG base t-cycles (4_194_304 Hz domain).
    fn tick(&mut self, tcycles: u8);
    fn cpu_tcycles_for_mcycles(&self, mcycles: u8) -> u8;
    fn pending_interrupts(&self) -> u8;
    fn interrupt_flags(&self) -> u8;
    fn set_interrupt_flags(&mut self, value: u8);
}
