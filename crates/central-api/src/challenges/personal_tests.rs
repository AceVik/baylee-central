use super::personal::*;

fn entry(n: u8) -> ExportEntry {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = n;
    ExportEntry::Participation {
        progress: TaskProgress {
            challenge: id,
            starts: 0,
            failures: 0,
            hint_level: 0,
            listed: false,
            first_win: None,
            completed_at: None,
            live_week: None,
            tactician: false,
            first_try: false,
            best_turn: None,
        },
    }
}

#[test]
fn export_cursor_requires_a_full_strictly_ascending_page_of_its_dataset() {
    let table = ExportTable::Participation;
    let page: Vec<_> = (1..=30).map(entry).collect();
    let last = page.last().unwrap().key().1;
    assert!(validate_page(table, None, &page, Some(last)).is_ok());
    assert!(validate_page(table, None, &page, None).is_ok());
    assert!(validate_page(table, Some(last), &[], None).is_ok());
    assert!(validate_page(table, Some(last), &[entry(31)], None).is_ok());
    assert!(validate_page(table, None, &page[..29], Some(last)).is_err());
    assert!(validate_page(table, None, &page, Some(entry(31).key().1)).is_err());
    assert!(validate_page(table, None, &[], Some(last)).is_err());
    assert!(validate_page(table, Some(last), &[entry(30)], None).is_err());
    assert!(validate_page(table, None, &[entry(2), entry(1)], None).is_err());
    assert!(validate_page(table, None, &[entry(1), entry(1)], None).is_err());
    assert!(validate_page(ExportTable::Attempts, None, &page, None).is_err());
    let mut oversized = page;
    oversized.push(entry(31));
    assert!(validate_page(table, None, &oversized, None).is_err());
    let mut invalid = entry(1);
    let ExportEntry::Participation { progress } = &mut invalid else {
        unreachable!()
    };
    progress.challenge[6] = 0x40;
    assert!(validate_page(table, None, &[invalid], None).is_err());
}
