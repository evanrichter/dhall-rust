#![no_main]
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeMap;

fuzz_target!(|data: &str| {
    let _: Result<BTreeMap<String, u64>, _> =
        serde_dhall::from_str(data).parse();
});
