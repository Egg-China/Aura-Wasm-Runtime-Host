use aura_wasm_guest::{AuraValue, ErrorCode, require_operation};

#[test]
fn guest_value_round_trip_is_canonical() {
    let bytes = AuraValue::Integer(42).to_wire().expect("encode value");
    assert_eq!(
        AuraValue::from_wire(&bytes).expect("decode value"),
        AuraValue::Integer(42)
    );
}

#[test]
fn operation_guard_rejects_a_different_operation_with_a_stable_code() {
    let error = require_operation("hook.after-game-launch", "hook.before-game-launch")
        .expect_err("reject different operation");
    assert_eq!(error.code(), ErrorCode::InvalidArgument);
}
