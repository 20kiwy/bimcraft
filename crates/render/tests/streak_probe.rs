//! Diagnostic probe: build 3D display lists for stress models and scan every
//! line/triangle for non-finite or absurd coordinates (the "streak" bug).

use bimcraft_bim::{formwork, sample, ElementData, ElementKind, Foundation, FoundationKind, FoundationSizes, Model, Opening, Wall};
use bimcraft_geom::Vec2;
use bimcraft_render::{build, ViewInfo, ViewModeKind};

fn info(show_formwork: bool, eye: bimcraft_geom::Vec3) -> ViewInfo {
    ViewInfo {
        mode: ViewModeKind::Perspective3D,
        active_level: 0,
        selection: vec![],
        half_extent: 20.0,
        ground_extent: 40.0,
        show_ceilings_3d: false,
        show_items_2d: true,
        light: true,
        show_formwork,
        eye,
    }
}

fn scan(label: &str, m: &Model, i: &ViewInfo) {
    let dl = build(m, i);
    let mut bad = 0;
    for l in &dl.lines {
        for (k, p) in [l.a, l.b].iter().enumerate() {
            let finite = p.x.is_finite() && p.y.is_finite() && p.z.is_finite();
            if !finite || p.z.abs() > 60.0 || p.x.abs() > 500.0 || p.y.abs() > 500.0 {
                bad += 1;
                if bad < 8 {
                    println!("[{label}] BAD line #{k} ({:.2},{:.2},{:.2}) -> ({:.2},{:.2},{:.2})", l.a.x, l.a.y, l.a.z, l.b.x, l.b.y, l.b.z);
                }
            }
        }
    }
    for t in &dl.tris {
        for p in &t.pts {
            let finite = p.x.is_finite() && p.y.is_finite() && p.z.is_finite();
            if !finite || p.z.abs() > 60.0 || p.x.abs() > 500.0 || p.y.abs() > 500.0 {
                bad += 1;
                if bad < 8 {
                    println!("[{label}] BAD tri ({:.2},{:.2},{:.2})", p.x, p.y, p.z);
                }
            }
        }
    }
    println!("[{label}] lines={} tris={} bad={}", dl.lines.len(), dl.tris.len(), bad);
    assert_eq!(bad, 0, "{label}: display list contains degenerate/huge geometry");
}

#[test]
fn no_streaks_in_typical_models() {
    let eye = bimcraft_geom::Vec3::new(14.0, -14.0, 18.0);

    // 1. Sample model, formwork on/off.
    let m = sample();
    scan("sample formwork=off", &m, &info(false, eye));
    scan("sample formwork=on", &m, &info(true, eye));

    // 2. Wall with "up to level" top constraint (2 levels).
    let mut m2 = Model::new();
    let l0 = m2.levels[0].id;
    let l1 = m2.add_level("Level 1", 3.0, 3.0);
    let wid = m2.add_wall(l0, vec![Vec2::ZERO, Vec2::new(8.0, 0.0)], 0.2, 3.0);
    if let Some(bimcraft_bim::Element { data: ElementData::Wall(w), .. }) = m2.element_mut(wid) {
        w.top_level = Some(l1);
    }
    scan("up-to-level wall", &m2, &info(true, eye));

    // 3. Foundation kinds seeded to walls.
    let mut m3 = sample();
    let lvl = m3.levels[0].id;
    for (name, kind) in
        [("Slab", FoundationKind::Slab), ("Stem", FoundationKind::StemSlab), ("Crawl", FoundationKind::Crawl), ("Base", FoundationKind::Basement)]
    {
        m3.add_foundation(Foundation { id: 0, level: lvl, kind, seed: Vec2::new(4.0, 3.0), sizes: FoundationSizes::default(), name: name.into() });
    }
    scan("foundations", &m3, &info(true, eye));

    // 4. Weird openings: s beyond wall length, zero/negative sizes, s at ends.
    let mut m4 = Model::new();
    let l0 = m4.levels[0].id;
    let wid = m4.add_wall(l0, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
    for (s, width, height, sill) in [(3.0, 1.2, 1.4, 0.9), (100.0, 1.0, 1.0, 0.0), (-5.0, 1.0, 1.0, 0.0), (0.0, 0.0, 0.0, 0.0), (6.0, 2.0, 5.0, 0.0)]
    {
        m4.add_opening(ElementKind::Window, Opening { wall: wid, s, width, height, sill, ..Default::default() });
    }
    scan("weird openings", &m4, &info(true, eye));

    // 5. Degenerate walls: zero length, duplicate points, single point.
    let mut m5 = Model::new();
    let l0 = m5.levels[0].id;
    m5.add_wall(l0, vec![Vec2::ZERO, Vec2::ZERO], 0.2, 3.0);
    m5.add_wall(l0, vec![Vec2::new(1.0, 1.0)], 0.2, 3.0);
    m5.add_wall(l0, vec![Vec2::new(2.0, 2.0), Vec2::new(2.0, 2.0), Vec2::new(3.0, 2.0)], 0.2, 3.0);
    m5.add_wall(l0, vec![Vec2::new(0.0, 5.0), Vec2::new(4.0, 5.0)], 0.0, 3.0); // zero thickness
    scan("degenerate walls", &m5, &info(true, eye));

    // 6. Formwork on a tall "up to level" wall chain.
    let mut m6 = Model::new();
    let l0 = m6.levels[0].id;
    let l1 = m6.add_level("L1", 3.0, 6.0);
    let twid = m6.add_wall(l0, vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 6.0)], 0.25, 3.0);
    let _ = Wall::default();
    if let Some(bimcraft_bim::Element { data: ElementData::Wall(w), .. }) = m6.element_mut(twid) {
        w.top_level = Some(l1);
    }
    let fw = formwork::level_formwork(&m6, l0);
    let tallest = fw.iter().flat_map(|(_, ps, _)| ps.iter()).map(|p| p.z1).fold(0.0_f64, f64::max);
    println!("[tall formwork] max panel z1 = {tallest}");
    scan("tall chain formwork", &m6, &info(true, eye));
}
