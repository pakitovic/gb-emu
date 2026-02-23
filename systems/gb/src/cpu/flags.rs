pub(super) fn set_flag_z(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 7;
    } else {
        *f &= !(1 << 7);
    }
}

pub(super) fn set_flag_n(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 6;
    } else {
        *f &= !(1 << 6);
    }
}

pub(super) fn set_flag_h(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 5;
    } else {
        *f &= !(1 << 5);
    }
}

pub(super) fn set_flag_c(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 4;
    } else {
        *f &= !(1 << 4);
    }
}

pub(super) fn get_flag_z(f: u8) -> bool {
    (f & (1 << 7)) != 0
}

pub(super) fn get_flag_n(f: u8) -> bool {
    (f & (1 << 6)) != 0
}

pub(super) fn get_flag_h(f: u8) -> bool {
    (f & (1 << 5)) != 0
}

pub(super) fn get_flag_c(f: u8) -> bool {
    (f & (1 << 4)) != 0
}
