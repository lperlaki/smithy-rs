/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

mod blob_serde;
#[allow(dead_code)]
mod doc_json;
#[allow(dead_code)]
mod error_code;
mod generic_error;
#[allow(dead_code)]
mod idempotency_token;
mod instant_epoch;
mod instant_httpdate;
mod instant_iso8601;

// This test is outside of uuid.rs to enable copying the entirety of uuid.rs into the SDK without
// requiring a proptest dependency
#[cfg(test)]
mod test {
    use crate::doc_json::SerDoc;
    use crate::idempotency_token::uuid_v4;
    use proptest::prelude::*;
    use proptest::std_facade::HashMap;
    use smithy_types::Document;
    use smithy_types::Number;

    #[test]
    fn nan_floats_serialize_null() {
        let mut map = HashMap::new();
        map.insert("num".to_string(), Document::Number(Number::PosInt(45)));
        map.insert("nan".to_string(), Document::Number(Number::Float(f64::NAN)));
        let doc = Document::Object(map);
        assert_eq!(
            serde_json::to_value(&SerDoc(&doc)).unwrap(),
            serde_json::json!({"num":45,"nan":null})
        );
    }

    #[test]
    fn test_uuid() {
        assert_eq!(uuid_v4(0), "00000000-0000-4000-8000-000000000000");
        assert_eq!(uuid_v4(12341234), "2ff4cb00-0000-4000-8000-000000000000");
        assert_eq!(
            uuid_v4(u128::max_value()),
            "ffffffff-ffff-4fff-ffff-ffffffffffff"
        );
    }

    fn assert_valid(uuid: String) {
        assert_eq!(uuid.len(), 36);
        let bytes = uuid.as_bytes();
        let dashes: Vec<usize> = uuid
            .chars()
            .enumerate()
            .filter_map(|(idx, chr)| if chr == '-' { Some(idx) } else { None })
            .collect();
        assert_eq!(dashes, vec![8, 13, 18, 23]);
        // Check version
        assert_eq!(bytes[14] as char, '4');
        // Check variant
        assert!(bytes[19] as char >= '8');
    }

    proptest! {
        #[test]
        fn doesnt_crash_uuid(v in any::<u128>()) {
            let uuid = uuid_v4(v);
            assert_valid(uuid);
        }
    }
}