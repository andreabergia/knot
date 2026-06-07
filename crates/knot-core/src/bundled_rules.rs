pub struct BundledRule {
    pub id: &'static str,
    pub wasm: &'static [u8],
}

pub const RULES: &[BundledRule] = &[BundledRule {
    id: "knot/ts-debugger",
    wasm: include_bytes!(concat!(env!("OUT_DIR"), "/ts-debugger.wasm")),
}];

#[cfg(test)]
mod tests {
    use knot_runtime::WasmRuntime;

    use super::*;

    #[test]
    fn bundled_rule_metadata_matches_registration() {
        let runtime = WasmRuntime::new();

        for rule in RULES {
            let metadata = runtime.metadata(rule.wasm).expect("metadata should load");

            assert_eq!(metadata.id, rule.id);
        }
    }
}
