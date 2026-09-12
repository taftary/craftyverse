use planet_crafter_engine::testing::required_capacity;

#[test]
fn fitting_request_keeps_the_current_capacity() {
    assert_eq!(required_capacity(10, 5), 10);
    assert_eq!(required_capacity(10, 10), 10);
}

#[test]
fn growth_doubles_when_that_covers_the_request() {
    assert_eq!(required_capacity(10, 15), 20);
}

#[test]
fn growth_jumps_to_the_request_when_doubling_falls_short() {
    assert_eq!(required_capacity(10, 25), 25);
    assert!(required_capacity(3, 100) >= 100);
}

#[test]
fn zero_start_grows_to_the_request() {
    assert_eq!(required_capacity(0, 5), 5);
    assert_eq!(required_capacity(0, 0), 0);
}
