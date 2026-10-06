use wire_app_engine::config::{retry_delay_secs, SYNC_RETRY_ATTEMPTS, SYNC_RETRY_MAX_SECS};

#[test]
fn retry_delays_double_from_five_seconds_and_cap_at_a_minute() {
    assert_eq!(retry_delay_secs(0), 5);
    assert_eq!(retry_delay_secs(1), 5);
    assert_eq!(retry_delay_secs(2), 10);
    assert_eq!(retry_delay_secs(3), 20);
    assert_eq!(retry_delay_secs(4), 40);
    assert_eq!(retry_delay_secs(5), SYNC_RETRY_MAX_SECS);
    assert_eq!(retry_delay_secs(u32::MAX), SYNC_RETRY_MAX_SECS);
    let budget: u64 = (1..=SYNC_RETRY_ATTEMPTS).map(retry_delay_secs).sum();
    assert_eq!(budget, 5 + 10 + 20 + 40 + 60 * 4);
}
