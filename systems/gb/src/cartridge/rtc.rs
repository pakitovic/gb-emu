#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Mbc3Rtc {
    seconds: u8,
    minutes: u8,
    hours: u8,
    day_counter: u16,
    carry: bool,
    halted: bool,
    latched_registers: [u8; 5],
    pub(super) has_latched_snapshot: bool,
    latch_armed: bool,
    last_update_epoch_secs: u64,
}

impl Mbc3Rtc {
    pub(super) fn new(now_epoch_secs: u64) -> Self {
        Self {
            seconds: 0,
            minutes: 0,
            hours: 0,
            day_counter: 0,
            carry: false,
            halted: false,
            latched_registers: [0; 5],
            has_latched_snapshot: false,
            latch_armed: false,
            last_update_epoch_secs: now_epoch_secs,
        }
    }

    pub(super) fn tick_to_epoch(&mut self, now_epoch_secs: u64) {
        if now_epoch_secs <= self.last_update_epoch_secs {
            return;
        }
        let elapsed = now_epoch_secs.saturating_sub(self.last_update_epoch_secs);
        self.last_update_epoch_secs = now_epoch_secs;
        if self.halted {
            return;
        }
        self.add_elapsed_seconds(elapsed);
    }

    fn add_elapsed_seconds(&mut self, elapsed_secs: u64) {
        if elapsed_secs == 0 {
            return;
        }

        let mut total = (self.seconds as u64)
            + (self.minutes as u64) * 60
            + (self.hours as u64) * 3600
            + elapsed_secs;

        let total_days = total / 86_400;
        total %= 86_400;

        self.hours = (total / 3600) as u8;
        total %= 3600;
        self.minutes = (total / 60) as u8;
        self.seconds = (total % 60) as u8;

        if total_days > 0 {
            let new_days = (self.day_counter as u64).saturating_add(total_days);
            if new_days > 0x01FF {
                self.carry = true;
            }
            self.day_counter = (new_days & 0x01FF) as u16;
        }
    }

    pub(super) fn latch_command(&mut self, value: u8) {
        if value == 0 {
            self.latch_armed = true;
            return;
        }
        if value == 1 && self.latch_armed {
            self.latch();
        }
        self.latch_armed = false;
    }

    fn latch(&mut self) {
        self.latched_registers = self.live_registers();
        self.has_latched_snapshot = true;
    }

    fn live_registers(&self) -> [u8; 5] {
        let day_low = (self.day_counter & 0x00FF) as u8;
        let day_high = ((self.day_counter >> 8) as u8) & 0x01;
        let halt_bit = if self.halted { 0x40 } else { 0x00 };
        let carry_bit = if self.carry { 0x80 } else { 0x00 };
        [
            self.seconds % 60,
            self.minutes % 60,
            self.hours % 24,
            day_low,
            day_high | halt_bit | carry_bit,
        ]
    }

    pub(super) fn live_registers_at_epoch(&self, now_epoch_secs: u64) -> [u8; 5] {
        let mut snapshot = *self;
        snapshot.tick_to_epoch(now_epoch_secs);
        snapshot.live_registers()
    }

    pub(super) fn read_register(&self, register_select: u8, use_latched: bool) -> u8 {
        let index = (register_select.saturating_sub(0x08)) as usize;
        if index >= 5 {
            return 0xFF;
        }
        if use_latched {
            self.latched_registers[index]
        } else {
            self.live_registers()[index]
        }
    }

    pub(super) fn write_register(&mut self, register_select: u8, value: u8, now_epoch_secs: u64) {
        self.tick_to_epoch(now_epoch_secs);
        match register_select {
            0x08 => self.seconds = value % 60,
            0x09 => self.minutes = value % 60,
            0x0A => self.hours = value % 24,
            0x0B => {
                self.day_counter = (self.day_counter & 0x0100) | value as u16;
            }
            0x0C => {
                self.day_counter = (self.day_counter & 0x00FF) | (((value & 0x01) as u16) << 8);
                self.halted = (value & 0x40) != 0;
                self.carry = (value & 0x80) != 0;
                self.last_update_epoch_secs = now_epoch_secs;
            }
            _ => {}
        }
    }

    pub(super) fn serialize(&mut self, now_epoch_secs: u64) -> [u8; 13] {
        self.tick_to_epoch(now_epoch_secs);
        let regs = self.live_registers();
        let mut out = [0u8; 13];
        out[0..5].copy_from_slice(&regs);
        out[5..13].copy_from_slice(&self.last_update_epoch_secs.to_le_bytes());
        out
    }

    pub(super) fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 13 {
            return None;
        }
        let mut last_update_raw = [0u8; 8];
        last_update_raw.copy_from_slice(&bytes[5..13]);
        let rtc = Self {
            seconds: bytes[0] % 60,
            minutes: bytes[1] % 60,
            hours: bytes[2] % 24,
            day_counter: ((bytes[4] as u16 & 0x01) << 8) | bytes[3] as u16,
            carry: (bytes[4] & 0x80) != 0,
            halted: (bytes[4] & 0x40) != 0,
            latched_registers: [0; 5],
            has_latched_snapshot: false,
            latch_armed: false,
            last_update_epoch_secs: u64::from_le_bytes(last_update_raw),
        };
        Some(rtc)
    }
}
