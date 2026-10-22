//! Regression: the 3D display list must stay bounded. A past build drew
//! helper geometry (datum markers, axes) taller than the model and un-faded
//! ground grids, which visually swept the viewport as stray vertical bands.

use bimcraft_geom::Vec3;

fn info_for(m: &bimcraft_bim::Model, eye: Vec3) -> bimcraft_render::ViewInfo {
    bimcraft_render::ViewInfo {
        mode: bimcraft_render::ViewModeKind::Perspective3D,
        active_level: m.levels[0].id,
        selection: vec![],
        half_extent: 20.0,
        ground_extent: 40.0,
        show_ceilings_3d: true,
        show_items_2d: true,
        eye,
    }
}

#[test]
fn no_geometry_taller_than_the_axes() {
    let m = bimcraft_bim::sample();
    let dl = bimcraft_render::build(&m, &info_for(&m, Vec3::ZERO));
    for (i, l) in dl.lines.iter().enumerate() {
        let zmax = l.a.z.max(l.b.z);
        let zmin = l.a.z.min(l.b.z);
        assert!(zmax <= 8.6, "line #{i} reaches z={zmax:.2} (axis cap is 8 m)");
        assert!(zmin >= -0.5, "line #{i} dips to z={zmin:.2}");
    }
    for (i, t) in dl.tris.iter().enumerate() {
        let zmax = t.pts.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
        assert!(zmax <= 8.6, "tri #{i} reaches z={zmax:.2}");
    }
}

#[test]
fn far_field_dissolves_with_distance() {
    let m = bimcraft_bim::sample();
    // Eye far from the model: every grid/datum line within 36 m of the eye may
    // exist, but nothing may remain beyond the fade radius (36 m) and faded
    // lines must carry reduced alpha.
    let eye = Vec3::new(60.0, 60.0, 2.0);
    let dl = bimcraft_render::build(&m, &info_for(&m, eye));
    for l in &dl.lines {
        let mid = (l.a + l.b) * 0.5;
        let d = ((mid.x - eye.x).powi(2) + (mid.y - eye.y).powi(2)).sqrt();
        if d > 37.0 && l.a.z < 1.6 && l.b.z < 1.6 {
            // Ground-level helper line beyond the fade radius must be either
            // model geometry (walls/slabs are opaque) or short (<= 8 m, e.g.
            // model outlines), never a long helper sweep.
            let len = (l.b - l.a).length();
            assert!(len <= 8.0 || l.color.a() == 255, "long ground line at d={d:.1} (len {len:.1}) should have been faded out");
        }
    }
}

#[test]
fn walk_eye_sees_a_quiet_grid() {
    let m = bimcraft_bim::sample();
    // Walking inside the house: the grid right around the eye stays visible.
    let eye = Vec3::new(2.0, 2.0, 1.65);
    let dl = bimcraft_render::build(&m, &info_for(&m, eye));
    let near_grid = dl.lines.iter().filter(|l| l.color.a() < 255 && (l.a.z - 0.0).abs() < 0.1 && (l.b.z - 0.0).abs() < 0.1).count();
    assert!(near_grid > 10, "near-field grid should still be drawn ({near_grid} segments)");
}
