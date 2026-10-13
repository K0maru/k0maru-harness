use k0maru_harness::probe::ComponentProbe;

#[test]
fn test_probe_buses() {
    let report = ComponentProbe::run_full_probe();
    assert!(!report.timestamp.is_empty());
    assert_eq!(report.components.len(), 4);
    assert_eq!(report.buses.len(), 2);
    // At least Linda tuple space and receipts should exist and be writable
    for bus in &report.buses {
        assert!(bus.exists, "Bus {} should exist", bus.name);
        assert!(bus.writable, "Bus {} should be writable", bus.name);
    }
}

#[test]
fn test_find_binary_fallback() {
    // Basic binaries like sh or ls should always be discoverable
    assert!(ComponentProbe::find_binary("sh").is_some());
    assert!(ComponentProbe::find_binary("this_binary_does_not_exist_xyz123").is_none());
}
