use apate::config::parser::parse_config;
use proptest::prelude::*;

proptest! {
    #[test]
    fn parser_never_panics(input in ".*") {
        let _ = parse_config(&input);
    }
}
