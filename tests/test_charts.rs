use chrono::{Datelike, NaiveDate};
use habit::ui::charts::{
    calculate_curve_points, calculate_loop_heatmap_layout, days_in_month, draw_progress_ring,
    heatmap_cell_rect, heatmap_hit_test, loop_heatmap_cell_rect, loop_heatmap_hit_test, Point,
    Rect, Rgba,
};

#[test]
fn test_loop_heatmap_cells_are_square_and_centered() {
    let rect = heatmap_cell_rect(0, 0, 400.0, 300.0);
    assert!(
        (rect.width - rect.height).abs() < 1e-6,
        "Cells must be square"
    );
    assert!(rect.width >= 16.0 && rect.width <= 40.0);
}

#[test]
fn test_days_in_month_and_leap_years() {
    // 31-day months
    assert_eq!(days_in_month(2026, 1), 31);
    assert_eq!(days_in_month(2026, 3), 31);
    assert_eq!(days_in_month(2026, 5), 31);
    assert_eq!(days_in_month(2026, 7), 31);
    assert_eq!(days_in_month(2026, 8), 31);
    assert_eq!(days_in_month(2026, 10), 31);
    assert_eq!(days_in_month(2026, 12), 31);

    // 30-day months
    assert_eq!(days_in_month(2026, 4), 30);
    assert_eq!(days_in_month(2026, 6), 30);
    assert_eq!(days_in_month(2026, 9), 30);
    assert_eq!(days_in_month(2026, 11), 30);

    // February non-leap years
    assert_eq!(days_in_month(2026, 2), 28);
    assert_eq!(days_in_month(2025, 2), 28);
    assert_eq!(days_in_month(2023, 2), 28);
    assert_eq!(days_in_month(1900, 2), 28); // Century not divisible by 400
    assert_eq!(days_in_month(2100, 2), 28);

    // February leap years
    assert_eq!(days_in_month(2024, 2), 29); // Divisible by 4
    assert_eq!(days_in_month(2028, 2), 29);
    assert_eq!(days_in_month(2000, 2), 29); // Century divisible by 400

    // Out of range month
    assert_eq!(days_in_month(2026, 0), 0);
    assert_eq!(days_in_month(2026, 13), 0);
}

#[test]
fn test_rgba_from_hex() {
    // Standard 6-digit hex
    let blue = Rgba::from_hex("#3584E4").expect("Valid blue hex");
    assert!((blue.r - 53.0 / 255.0).abs() < 1e-4);
    assert!((blue.g - 132.0 / 255.0).abs() < 1e-4);
    assert!((blue.b - 228.0 / 255.0).abs() < 1e-4);
    assert!((blue.a - 1.0).abs() < 1e-4);

    let orange = Rgba::from_hex("#FF7800").expect("Valid orange hex");
    assert!((orange.r - 1.0).abs() < 1e-4);
    assert!((orange.g - 120.0 / 255.0).abs() < 1e-4);
    assert!((orange.b - 0.0).abs() < 1e-4);
    assert!((orange.a - 1.0).abs() < 1e-4);

    let black = Rgba::from_hex("#000000").expect("Valid black hex");
    assert!((black.r - 0.0).abs() < 1e-4);
    assert!((black.g - 0.0).abs() < 1e-4);
    assert!((black.b - 0.0).abs() < 1e-4);
    assert!((black.a - 1.0).abs() < 1e-4);

    // 8-digit hex with alpha
    let semi_white = Rgba::from_hex("#FFFFFF80").expect("Valid 8-digit hex");
    assert!((semi_white.r - 1.0).abs() < 1e-4);
    assert!((semi_white.g - 1.0).abs() < 1e-4);
    assert!((semi_white.b - 1.0).abs() < 1e-4);
    assert!((semi_white.a - 128.0 / 255.0).abs() < 1e-4);

    // Without leading '#'
    let no_hash = Rgba::from_hex("3584E4").expect("Valid without hash");
    assert_eq!(blue, no_hash);

    // Lowercase hex
    let lower = Rgba::from_hex("#3584e4").expect("Valid lowercase");
    assert_eq!(blue, lower);

    // Invalid hex formats
    assert!(Rgba::from_hex("").is_none());
    assert!(Rgba::from_hex("#").is_none());
    assert!(Rgba::from_hex("#12345").is_none());
    assert!(Rgba::from_hex("#1234567").is_none());
    assert!(Rgba::from_hex("#123456789").is_none());
    assert!(Rgba::from_hex("#GGGGGG").is_none());

    // Multi-byte non-ASCII characters with byte length 6 or 8 must safely return None
    assert!(Rgba::from_hex("#🦀12").is_none());
    assert!(Rgba::from_hex("#🦀1234").is_none());
}

#[test]
fn test_calculate_curve_points() {
    let width = 300.0;
    let height = 150.0;
    let padding = 10.0;

    // Empty data produces empty points
    let empty_pts = calculate_curve_points(&[], width, height, padding);
    assert_eq!(empty_pts, []);

    // Single point: centered or within padding bounds
    let single_pts = calculate_curve_points(&[0.5], width, height, padding);
    assert_eq!(single_pts.len(), 1);
    assert!(single_pts[0].x >= padding && single_pts[0].x <= width - padding);
    assert!(single_pts[0].y >= padding && single_pts[0].y <= height - padding);

    // Multiple points
    let data = [0.0, 0.5, 1.0, 0.25];
    let pts = calculate_curve_points(&data, width, height, padding);
    assert_eq!(pts.len(), 4);

    // X coordinates monotonically increase from padding to width - padding
    assert!((pts[0].x - padding).abs() < 1e-4);
    assert!((pts[3].x - (width - padding)).abs() < 1e-4);
    assert!(pts[1].x > pts[0].x);
    assert!(pts[2].x > pts[1].x);
    assert!(pts[3].x > pts[2].x);

    // Y coordinate mapping: 1.0 is at top (padding), 0.0 is at bottom (height - padding)
    let y_min = padding;
    let y_max = height - padding;
    assert!(
        (pts[0].y - y_max).abs() < 1e-4,
        "Value 0.0 should be at bottom"
    );
    assert!(
        (pts[2].y - y_min).abs() < 1e-4,
        "Value 1.0 should be at top"
    );

    // All points remain within bounds
    for pt in &pts {
        assert!(
            pt.x >= 0.0 && pt.x <= width,
            "X point {} out of bounds",
            pt.x
        );
        assert!(
            pt.y >= 0.0 && pt.y <= height,
            "Y point {} out of bounds",
            pt.y
        );
    }

    // Values outside [0.0, 1.0] are clamped
    let clamped_pts = calculate_curve_points(&[-0.5, 1.5], width, height, padding);
    assert_eq!(clamped_pts.len(), 2);
    assert!((clamped_pts[0].y - y_max).abs() < 1e-4);
    assert!((clamped_pts[1].y - y_min).abs() < 1e-4);
}

#[test]
#[allow(clippy::similar_names)]
fn test_heatmap_hit_test_identifies_correct_date() {
    let width = 200.0;
    let height = 150.0;

    // October 2026: 2026-10-01 is Thursday (Monday=0 -> Thursday is col 3)
    let oct_1_rect = heatmap_cell_rect(3, 0, width, height);
    let hit_oct_1 = heatmap_hit_test(
        oct_1_rect.x + oct_1_rect.width / 2.0,
        oct_1_rect.y + oct_1_rect.height / 2.0,
        width,
        height,
        2026,
        10,
    );
    assert_eq!(
        hit_oct_1,
        Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap())
    );

    // 2026-10-05 is Monday (row 1, col 0)
    let oct_5_rect = heatmap_cell_rect(0, 1, width, height);
    let hit_oct_5 = heatmap_hit_test(
        oct_5_rect.x + oct_5_rect.width / 2.0,
        oct_5_rect.y + oct_5_rect.height / 2.0,
        width,
        height,
        2026,
        10,
    );
    assert_eq!(
        hit_oct_5,
        Some(NaiveDate::from_ymd_opt(2026, 10, 5).unwrap())
    );

    // 2026-10-31 is Saturday:
    // Offset from Oct 1 (col 3) is 3 + 30 = 33.
    // 33 / 7 = 4 (row 4), 33 % 7 = 5 (col 5).
    let oct_31_rect = heatmap_cell_rect(5, 4, width, height);
    let hit_oct_31 = heatmap_hit_test(
        oct_31_rect.x + oct_31_rect.width / 2.0,
        oct_31_rect.y + oct_31_rect.height / 2.0,
        width,
        height,
        2026,
        10,
    );
    assert_eq!(
        hit_oct_31,
        Some(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap())
    );

    // Test a month starting on Monday: June 2026 (2026-06-01 is Monday, col 0 row 0)
    let jun_1_rect = heatmap_cell_rect(0, 0, width, height);
    let hit_jun_1 = heatmap_hit_test(
        jun_1_rect.x + jun_1_rect.width / 2.0,
        jun_1_rect.y + jun_1_rect.height / 2.0,
        width,
        height,
        2026,
        6,
    );
    assert_eq!(
        hit_jun_1,
        Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap())
    );
    assert_eq!(hit_jun_1.unwrap().month(), 6);
}

#[test]
fn test_heatmap_hit_test_out_of_bounds_and_gaps() {
    let width = 200.0;
    let height = 150.0;

    // 1. Before 1st of month: October 2026 starts on Thursday (col 3).
    // Cols 0, 1, 2 in row 0 are before Oct 1.
    for col in 0..3 {
        let cell = heatmap_cell_rect(col, 0, width, height);
        let hit = heatmap_hit_test(
            cell.x + cell.width / 2.0,
            cell.y + cell.height / 2.0,
            width,
            height,
            2026,
            10,
        );
        assert_eq!(hit, None, "Expected None for col {col} row 0 before Oct 1");
    }

    // 2. Past last day of month: October 31 is row 4, col 5.
    // Row 4 col 6 is after Oct 31.
    let cell_after_31 = heatmap_cell_rect(6, 4, width, height);
    let hit_after_31 = heatmap_hit_test(
        cell_after_31.x + cell_after_31.width / 2.0,
        cell_after_31.y + cell_after_31.height / 2.0,
        width,
        height,
        2026,
        10,
    );
    assert_eq!(hit_after_31, None, "Expected None for day after Oct 31");

    // Row 5 is entirely after Oct 31
    for col in 0..7 {
        let cell = heatmap_cell_rect(col, 5, width, height);
        let hit = heatmap_hit_test(
            cell.x + cell.width / 2.0,
            cell.y + cell.height / 2.0,
            width,
            height,
            2026,
            10,
        );
        assert_eq!(hit, None, "Expected None for row 5 in Oct 2026");
    }

    // 3. Cell spacing gap (between col 3 and col 4 on row 0)
    let cell_col3 = heatmap_cell_rect(3, 0, width, height);
    let cell_col4 = heatmap_cell_rect(4, 0, width, height);
    let gap_x = (cell_col3.x + cell_col3.width + cell_col4.x) / 2.0;
    let cell_y = cell_col3.y + cell_col3.height / 2.0;
    let hit_gap_x = heatmap_hit_test(gap_x, cell_y, width, height, 2026, 10);
    assert_eq!(hit_gap_x, None, "Point in horizontal gap must return None");

    // Vertical spacing gap (between row 0 and row 1 on col 3)
    let cell_row0 = heatmap_cell_rect(3, 0, width, height);
    let cell_row1 = heatmap_cell_rect(3, 1, width, height);
    let gap_y = (cell_row0.y + cell_row0.height + cell_row1.y) / 2.0;
    let cell_x = cell_row0.x + cell_row0.width / 2.0;
    let hit_gap_y = heatmap_hit_test(cell_x, gap_y, width, height, 2026, 10);
    assert_eq!(hit_gap_y, None, "Point in vertical gap must return None");

    // 4. Outside boundaries
    assert_eq!(heatmap_hit_test(-5.0, 50.0, width, height, 2026, 10), None);
    assert_eq!(heatmap_hit_test(50.0, -5.0, width, height, 2026, 10), None);
    assert_eq!(heatmap_hit_test(205.0, 50.0, width, height, 2026, 10), None);
    assert_eq!(heatmap_hit_test(50.0, 155.0, width, height, 2026, 10), None);

    // In canvas outer padding (e.g. top-left corner before cell 0,0)
    assert_eq!(heatmap_hit_test(2.0, 2.0, width, height, 2026, 10), None);
}

#[test]
fn test_geometry_types_and_helpers() {
    let pt = Point { x: 12.5, y: 34.5 };
    assert_eq!(pt.x, 12.5);
    assert_eq!(pt.y, 34.5);

    let rect = Rect {
        x: 10.0,
        y: 20.0,
        width: 30.0,
        height: 40.0,
    };
    assert!(rect.contains(15.0, 25.0));
    assert!(rect.contains(10.0, 20.0));
    assert!(rect.contains(40.0, 60.0));
    assert!(!rect.contains(5.0, 25.0));
    assert!(!rect.contains(15.0, 15.0));
    assert!(!rect.contains(45.0, 25.0));
    assert!(!rect.contains(15.0, 65.0));
}

#[test]
fn test_loop_heatmap_layout_and_cell_geometry() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(); // Tuesday
    let width = 480.0;
    let height = 160.0;

    let layout = calculate_loop_heatmap_layout(width, height, today, 0);
    assert_eq!(layout.padding, 8.0);
    assert!(layout.square_size >= 14.0 && layout.square_size <= 20.0);
    assert_eq!(layout.cell_size, layout.square_size - layout.square_spacing);
    assert!(layout.n_cols >= 20);

    // Latest column Monday + 1 day should be Tuesday (today)
    let last_col = layout.n_cols - 1;
    let last_monday = layout.top_left_monday + chrono::Duration::weeks(last_col as i64);
    assert_eq!(last_monday + chrono::Duration::days(1), today);

    // Cell rect must be square
    let cell = loop_heatmap_cell_rect(0, 0, &layout);
    assert!((cell.width - cell.height).abs() < 1e-6);
    assert_eq!(cell.width, layout.cell_size);

    // Week offset moves top_left_monday back by exact weeks
    let layout_offset_4 = calculate_loop_heatmap_layout(width, height, today, 4);
    assert_eq!(
        layout_offset_4.top_left_monday,
        layout.top_left_monday - chrono::Duration::weeks(4)
    );
}

#[test]
fn test_loop_heatmap_hit_test_today_and_past() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(); // Tuesday (row 1)
    let width = 480.0;
    let height = 160.0;

    let layout = calculate_loop_heatmap_layout(width, height, today, 0);
    let last_col = layout.n_cols - 1;

    // Hit test on today: last_col, row 1
    let today_rect = loop_heatmap_cell_rect(last_col, 1, &layout);
    let hit_today = loop_heatmap_hit_test(
        today_rect.x + today_rect.width / 2.0,
        today_rect.y + today_rect.height / 2.0,
        width,
        height,
        today,
        0,
    );
    assert_eq!(hit_today, Some(today));

    // Hit test on yesterday (Monday = row 0 of last_col)
    let mon_rect = loop_heatmap_cell_rect(last_col, 0, &layout);
    let hit_mon = loop_heatmap_hit_test(
        mon_rect.x + mon_rect.width / 2.0,
        mon_rect.y + mon_rect.height / 2.0,
        width,
        height,
        today,
        0,
    );
    assert_eq!(hit_mon, Some(today - chrono::Duration::days(1)));

    // Future days (Wednesday = row 2 of last_col) must return None
    let wed_rect = loop_heatmap_cell_rect(last_col, 2, &layout);
    let hit_wed = loop_heatmap_hit_test(
        wed_rect.x + wed_rect.width / 2.0,
        wed_rect.y + wed_rect.height / 2.0,
        width,
        height,
        today,
        0,
    );
    assert_eq!(hit_wed, None, "Future day after today must return None");

    // Hit test in spacing gap between row 0 and row 1 must return None
    let gap_y = (mon_rect.y + mon_rect.height + today_rect.y) / 2.0;
    let hit_gap = loop_heatmap_hit_test(
        today_rect.x + today_rect.width / 2.0,
        gap_y,
        width,
        height,
        today,
        0,
    );
    assert_eq!(hit_gap, None, "Gap must return None");

    // Hit test outside canvas must return None
    assert_eq!(
        loop_heatmap_hit_test(-10.0, 50.0, width, height, today, 0),
        None
    );
    assert_eq!(
        loop_heatmap_hit_test(50.0, -10.0, width, height, today, 0),
        None
    );
    assert_eq!(
        loop_heatmap_hit_test(500.0, 50.0, width, height, today, 0),
        None
    );
}

#[cfg(feature = "ui")]
#[test]
fn test_draw_progress_ring() {
    let surface = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 40, 40)
        .expect("Failed to create test image surface");
    let cr = gtk4::cairo::Context::new(&surface).expect("Failed to create cairo context");
    let color = Rgba::new(0.2, 0.6, 1.0, 1.0);

    // 1. Standard 50% progress
    draw_progress_ring(&cr, 40.0, 40.0, 0.5, &color);

    // 2. 0% progress
    draw_progress_ring(&cr, 40.0, 40.0, 0.0, &color);

    // 3. 100% progress
    draw_progress_ring(&cr, 40.0, 40.0, 1.0, &color);

    // 4. Clamping bounds: negative progress clamped to 0, > 1.0 clamped to 1.0
    draw_progress_ring(&cr, 40.0, 40.0, -0.25, &color);
    draw_progress_ring(&cr, 40.0, 40.0, 2.5, &color);

    // 5. Zero / negative dimension safety
    draw_progress_ring(&cr, 0.0, 40.0, 0.5, &color);
    draw_progress_ring(&cr, 40.0, -5.0, 0.5, &color);
}
