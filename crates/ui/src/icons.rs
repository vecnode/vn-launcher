//! Icons, drawn rather than imported.
//!
//! Every glyph is a handful of lines, circles and arcs in a 24-unit grid, painted
//! with the vector primitives egui already has. Shipping art would mean bundling
//! SVG or PNG assets and an image decoder for a dozen twelve-pixel marks; drawing
//! them costs a few hundred lines, stays crisp at any display scale, and takes its
//! colour from the theme like any other text.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

/// The icons the interface draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// A magnifying glass: search.
    Search,
    /// A circular arrow: check for updates.
    Refresh,
    /// A downward arrow into a tray: install.
    Download,
    /// A box with an arrow leaving it: open externally.
    External,
    /// A tick: done, up to date.
    Check,
    /// A circled exclamation: something is wrong.
    Alert,
    /// A circled i: information.
    Info,
    /// A parcel: an application.
    Package,
    /// Three sliders: settings.
    Sliders,
    /// A right-pointing angle bracket.
    ChevronRight,
    /// A left-pointing angle bracket.
    ChevronLeft,
    /// A cross: close or dismiss.
    Close,
    /// A folder: open a location.
    Folder,
    /// A waste bin: uninstall.
    Trash,
    /// A plus: add.
    Plus,
    /// A right-pointing triangle: start.
    Play,
    /// A filled circle: a status dot.
    Dot,
}

/// Draw an icon so that it fills `rect`.
///
/// `width` is the stroke width in points; `1.5` reads well at 16–20 pixels.
pub fn paint(painter: &Painter, icon: Icon, rect: Rect, color: Color32, width: f32) {
    let canvas = Canvas { painter, rect };
    let stroke = Stroke::new(width, color);
    let side = rect.width().min(rect.height());

    match icon {
        Icon::Search => {
            canvas.circle((10.5, 10.5), 6.4, stroke);
            canvas.line((15.3, 15.3), (20.0, 20.0), stroke);
        }
        Icon::Refresh => {
            // An almost-complete circle, with a filled head where it stops.
            canvas.arc((12.0, 12.0), 7.6, -55.0, 245.0, stroke, 40);
            let tip = polar((12.0, 12.0), 7.6, 245.0);
            let tangent = (-(245.0_f32).to_radians().sin(), (245.0_f32).to_radians().cos());
            canvas.arrow_head(tip, tangent, 3.6, color);
        }
        Icon::Download => {
            canvas.line((12.0, 3.2), (12.0, 14.6), stroke);
            canvas.path(&[(7.6, 10.2), (12.0, 14.6), (16.4, 10.2)], stroke, false);
            canvas.path(
                &[(4.0, 18.0), (4.0, 20.4), (20.0, 20.4), (20.0, 18.0)],
                stroke,
                false,
            );
        }
        Icon::External => {
            canvas.line((12.6, 11.4), (20.0, 4.0), stroke);
            canvas.path(&[(14.4, 4.0), (20.0, 4.0), (20.0, 9.6)], stroke, false);
            canvas.path(
                &[(18.4, 14.0), (18.4, 20.4), (3.6, 20.4), (3.6, 5.6), (10.0, 5.6)],
                stroke,
                false,
            );
        }
        Icon::Check => {
            canvas.path(&[(4.2, 12.6), (9.6, 18.0), (19.8, 6.4)], stroke, false);
        }
        Icon::Alert => {
            canvas.circle((12.0, 12.0), 8.4, stroke);
            canvas.line((12.0, 7.2), (12.0, 13.4), stroke);
            canvas.dot((12.0, 16.8), side * 0.045, color);
        }
        Icon::Info => {
            canvas.circle((12.0, 12.0), 8.4, stroke);
            canvas.dot((12.0, 7.6), side * 0.045, color);
            canvas.line((12.0, 11.0), (12.0, 17.0), stroke);
        }
        Icon::Package => {
            canvas.path(
                &[
                    (12.0, 2.8),
                    (20.8, 7.4),
                    (20.8, 16.6),
                    (12.0, 21.2),
                    (3.2, 16.6),
                    (3.2, 7.4),
                ],
                stroke,
                true,
            );
            canvas.path(&[(3.2, 7.4), (12.0, 12.0), (20.8, 7.4)], stroke, false);
            canvas.line((12.0, 12.0), (12.0, 21.2), stroke);
        }
        Icon::Sliders => {
            canvas.line((3.0, 7.6), (21.0, 7.6), stroke);
            canvas.line((3.0, 12.0), (21.0, 12.0), stroke);
            canvas.line((3.0, 16.4), (21.0, 16.4), stroke);
            canvas.dot((9.0, 7.6), side * 0.10, color);
            canvas.dot((15.6, 12.0), side * 0.10, color);
            canvas.dot((7.0, 16.4), side * 0.10, color);
        }
        Icon::ChevronRight => {
            canvas.path(&[(9.6, 5.0), (16.6, 12.0), (9.6, 19.0)], stroke, false);
        }
        Icon::ChevronLeft => {
            canvas.path(&[(14.4, 5.0), (7.4, 12.0), (14.4, 19.0)], stroke, false);
        }
        Icon::Close => {
            canvas.line((6.6, 6.6), (17.4, 17.4), stroke);
            canvas.line((17.4, 6.6), (6.6, 17.4), stroke);
        }
        Icon::Folder => {
            canvas.path(
                &[
                    (3.0, 19.4),
                    (3.0, 6.0),
                    (9.4, 6.0),
                    (11.4, 8.8),
                    (21.0, 8.8),
                    (21.0, 19.4),
                ],
                stroke,
                true,
            );
        }
        Icon::Trash => {
            canvas.line((4.0, 6.8), (20.0, 6.8), stroke);
            canvas.path(&[(9.2, 6.8), (9.2, 4.2), (14.8, 4.2), (14.8, 6.8)], stroke, false);
            canvas.path(
                &[(6.0, 6.8), (7.1, 20.8), (16.9, 20.8), (18.0, 6.8)],
                stroke,
                false,
            );
            canvas.line((10.2, 10.4), (10.2, 17.2), stroke);
            canvas.line((13.8, 10.4), (13.8, 17.2), stroke);
        }
        Icon::Plus => {
            canvas.line((12.0, 4.6), (12.0, 19.4), stroke);
            canvas.line((4.6, 12.0), (19.4, 12.0), stroke);
        }
        Icon::Play => {
            canvas.painter.add(Shape::convex_polygon(
                vec![
                    canvas.point(8.2, 4.8),
                    canvas.point(19.0, 12.0),
                    canvas.point(8.2, 19.2),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Dot => {
            canvas.dot((12.0, 12.0), side * 0.22, color);
        }
    }
}

/// Allocate a square and draw an icon in it.
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint(ui.painter(), icon, rect, color, width);
    }
    response
}

/// A point on a circle, in the 24-unit grid.
fn polar(centre: (f32, f32), radius: f32, degrees: f32) -> (f32, f32) {
    let radians = degrees.to_radians();
    (
        centre.0 + radius * radians.cos(),
        centre.1 + radius * radians.sin(),
    )
}

/// Maps the 24-unit design grid onto a screen rectangle.
struct Canvas<'a> {
    painter: &'a Painter,
    rect: Rect,
}

impl Canvas<'_> {
    /// A design-grid coordinate as a screen position.
    fn point(&self, x: f32, y: f32) -> Pos2 {
        pos2(
            self.rect.left() + x / 24.0 * self.rect.width(),
            self.rect.top() + y / 24.0 * self.rect.height(),
        )
    }

    /// A straight line between two design-grid points.
    fn line(&self, from: (f32, f32), to: (f32, f32), stroke: Stroke) {
        self.painter
            .line_segment([self.point(from.0, from.1), self.point(to.0, to.1)], stroke);
    }

    /// A polyline, optionally closed.
    fn path(&self, points: &[(f32, f32)], stroke: Stroke, closed: bool) {
        let screen: Vec<Pos2> = points.iter().map(|(x, y)| self.point(*x, *y)).collect();
        if screen.len() < 2 {
            return;
        }
        self.painter.add(if closed {
            Shape::closed_line(screen, stroke)
        } else {
            Shape::line(screen, stroke)
        });
    }

    /// A stroked circle.
    fn circle(&self, centre: (f32, f32), radius: f32, stroke: Stroke) {
        self.painter.circle_stroke(
            self.point(centre.0, centre.1),
            radius / 24.0 * self.rect.width(),
            stroke,
        );
    }

    /// A filled circle.
    fn dot(&self, centre: (f32, f32), radius: f32, color: Color32) {
        self.painter
            .circle_filled(self.point(centre.0, centre.1), radius, color);
    }

    /// A circular arc, drawn as a polyline so it needs no special shape.
    fn arc(
        &self,
        centre: (f32, f32),
        radius: f32,
        start_degrees: f32,
        end_degrees: f32,
        stroke: Stroke,
        segments: usize,
    ) {
        let segments = segments.max(2);
        let points: Vec<(f32, f32)> = (0..=segments)
            .map(|step| {
                let t = step as f32 / segments as f32;
                polar(centre, radius, start_degrees + (end_degrees - start_degrees) * t)
            })
            .collect();
        self.path(&points, stroke, false);
    }

    /// A filled triangular head pointing along `direction`.
    fn arrow_head(&self, tip: (f32, f32), direction: (f32, f32), size: f32, color: Color32) {
        let length = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
        if length <= f32::EPSILON {
            return;
        }
        let (dx, dy) = (direction.0 / length, direction.1 / length);
        let base = (tip.0 - dx * size, tip.1 - dy * size);
        let half = size * 0.52;
        let left = (base.0 - dy * half, base.1 + dx * half);
        let right = (base.0 + dy * half, base.1 - dx * half);
        self.painter.add(Shape::convex_polygon(
            vec![
                self.point(tip.0, tip.1),
                self.point(left.0, left.1),
                self.point(right.0, right.1),
            ],
            color,
            Stroke::NONE,
        ));
    }
}
