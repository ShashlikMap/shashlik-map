use geo::BoundingRect;
use geo_types::{Coord, LineString, Polygon};
#[derive(Copy, Clone)]
enum Side {
    Low,
    High,
}

pub(crate) fn subdivide_grid(poly: Polygon<f32>, level: u32) -> Vec<Polygon<f32>> {
    let mut out = vec![];
    if level <= 1 {
        out.push(poly);
        return out;
    }

    let bbox = match poly.bounding_rect() {
        Some(r) => r,
        None => return out,
    };
    let (min, max) = (bbox.min(), bbox.max());
    let bbox_width = (max.x - min.x) / level as f32;
    let bbox_height = (max.y - min.y) / level as f32;
    if !(bbox_width > 0.0) || !(bbox_height > 0.0) {
        out.push(poly);
        return out;
    }

    let cutter = |axis: u8, origin: f32, size: f32, poly: Polygon<f32>| -> Vec<Polygon<f32>> {
        let mut out = vec![];
        let mut rest = Some(poly);
        for i in 1..level {
            let cur = match rest.take() {
                Some(c) => c,
                None => break,
            };
            let k = origin + i as f32 * size;
            if let Some(lo) = clip_poly(&cur, axis, k, Side::Low) {
                out.push(lo);
            }
            rest = clip_poly(&cur, axis, k, Side::High);
        }
        if let Some(last) = rest {
            out.push(last);
        }
        out
    };

    let cols = cutter(0, min.x, bbox_width, poly);
    for col_poly in cols {
        out.extend(cutter(1, min.y, bbox_height, col_poly));
    }
    out
}

fn clip_poly(poly: &Polygon<f32>, axis: u8, k: f32, side: Side) -> Option<Polygon<f32>> {
    let exterior = clip_ring(&poly.exterior(), axis, k, side)?;
    let holes = poly
        .interiors()
        .iter()
        .flat_map(|ring| clip_ring(ring, axis, k, side))
        .map(LineString::new)
        .collect();
    Some(Polygon::new(LineString::from(exterior), holes))
}

fn clip_ring(ring: &LineString<f32>, axis: u8, k: f32, side: Side) -> Option<Vec<Coord<f32>>> {
    let mut out = vec![];

    let ring = open_ring(ring);
    if ring.len() < 3 {
        return None;
    }

    let c = |p: &Coord<f32>| if axis == 0 { p.x } else { p.y };
    let inside = |p: &Coord<f32>| match side {
        Side::Low => c(p) <= k,
        Side::High => c(p) >= k,
    };

    fn push(out: &mut Vec<Coord<f32>>, p: Coord<f32>) {
        if out.last().map_or(true, |l| l.x != p.x || l.y != p.y) {
            out.push(p);
        }
    }

    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let (ai, bi) = (inside(&a), inside(&b));

        if ai {
            push(&mut out, a);
        }
        if ai != bi {
            let (ca, cb) = (c(&a), c(&b));
            let t = (k - ca) / (cb - ca);
            let mut p = Coord {
                x: a.x + (b.x - a.x) * t,
                y: a.y + (b.y - a.y) * t,
            };
            if axis == 0 {
                p.x = k
            } else {
                p.y = k
            }
            push(&mut out, p);
        }
    }

    if out.len() > 1 && out[0] == *out.last().unwrap() {
        out.pop();
    }
    Some(out)
}

fn open_ring(ls: &LineString<f32>) -> &[Coord<f32>] {
    if ls.is_closed()
        && let [all_but_last @ .., _last] = &ls.0[..]
    {
        all_but_last
    } else {
        &ls.0
    }
}
