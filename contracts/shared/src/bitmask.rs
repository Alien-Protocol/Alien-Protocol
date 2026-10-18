/// Returns a mask with the given bit set.
pub fn set_flag(mask: u32, bit: u32) -> u32 {
    mask | bit
}

/// Returns a mask with the given bit cleared.
pub fn clear_flag(mask: u32, bit: u32) -> u32 {
    mask & !bit
}

/// Returns whether the given bit is currently set in the mask.
pub fn is_set(mask: u32, bit: u32) -> bool {
    mask & bit != 0
}

#[cfg(test)]
mod tests {
    use super::{clear_flag, is_set, set_flag};

    #[test]
    fn set_flag_adds_a_bit_without_touching_others() {
        assert_eq!(set_flag(0b0000, 0b0001), 0b0001);
        assert_eq!(set_flag(0b0101, 0b0010), 0b0111);
    }

    #[test]
    fn clear_flag_removes_only_target_bit() {
        assert_eq!(clear_flag(0b0111, 0b0010), 0b0101);
        assert_eq!(clear_flag(0b0000, 0b1000), 0b0000);
    }

    #[test]
    fn is_set_reports_bit_state() {
        assert!(!is_set(0b1010, 0b0001));
        assert!(is_set(0b1010, 0b0010));
        assert!(is_set(0b1010, 0b1000));
    }
}
