#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut host = muc::vm::FuzzHost;
    let _ = muc::vm::run_bytecode_with_fuel_and_host(data, &[], 256, &mut host);
});
