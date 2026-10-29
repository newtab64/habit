use chrono::NaiveDate;
use habit::ui::charts::{
    calculate_curve_points, calculate_loop_heatmap_layout, loop_heatmap_cell_rect,
    loop_heatmap_hit_test, Point, Rect, Rgba,
};

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
    let last_col_i64 = i64::try_from(last_col).unwrap();
    let last_monday = layout.top_left_monday + chrono::Duration::weeks(last_col_i64);
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
    use habit::ui::charts::draw_progress_ring;
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
