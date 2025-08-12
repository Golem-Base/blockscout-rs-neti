use serde_json::Value;

/// Assert specific fields in JSON object
pub fn assert_fields(actual: &Value, expected: Value) {
    let expected = expected
        .as_object()
        .expect("expected_fields has to be an object");

    for (key, expected_value) in expected {
        let actual_value = &actual[key];

        assert_eq!(
            actual_value, expected_value,
            "Field '{key}' assertion failed \nExpected: {expected_value}\nActual: {actual_value}"
        );
    }
}

/// Assert specific fields in JSON array of objects
pub fn assert_fields_array(actual: &Value, expected: Vec<Value>) {
    let actual = actual.as_array().expect("actual has to be an array");

    assert_eq!(
        actual.len(),
        expected.len(),
        "different lengths of actual and expected items",
    );

    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert_fields(actual, expected.clone());
    }
}
