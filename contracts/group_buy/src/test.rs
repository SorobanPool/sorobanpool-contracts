use super::*;
use soroban_sdk::symbol_short;

#[test]
fn reject_reason_allow_list() {
    // Allowed reasons
    assert!(is_valid_reject_reason(&symbol_short!("no_stock")));
    assert!(is_valid_reject_reason(&symbol_short!("price_chg")));
    assert!(is_valid_reject_reason(&symbol_short!("quality")));
    assert!(is_valid_reject_reason(&symbol_short!("other")));

    // Disallowed reasons
    assert!(!is_valid_reject_reason(&symbol_short!("invalid")));
    assert!(!is_valid_reject_reason(&symbol_short!("random")));
    assert!(!is_valid_reject_reason(&symbol_short!("foo")));
}
