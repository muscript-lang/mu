#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    let parsed = muc::parser::parse_str(&s);
    let formatted = muc::fmt::parse_and_format(&s);

    if let Ok(program) = parsed {
        let first = muc::fmt::format_program(&program);
        if let Ok(reparsed) = muc::parser::parse_str(&first) {
            let second = muc::fmt::format_program(&reparsed);
            assert_eq!(first, second);
        }
    }

    if let Ok(out) = formatted {
        if let Ok(reparsed) = muc::parser::parse_str(&out) {
            let second = muc::fmt::format_program(&reparsed);
            assert_eq!(out, second);
        }
    }
});
