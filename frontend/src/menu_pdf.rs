//! Server-side generation of a printable PDF menu sheet.
//!
//! Renders the restaurant logo and a menu title, followed by every category
//! (main courses first). Each category gets a full-width header band and then
//! its items in a two-column list with a vertical divider between the columns;
//! main-course items additionally show a thumbnail and description. Leader dots
//! connect each item name to its right-aligned price.
//!
//! Layout is done by hand on an A4 portrait page with a top-down cursor (`y` is
//! the distance from the top edge in millimetres); categories and their rows
//! flow onto new pages when they would overflow the bottom margin. printpdf
//! places everything from the bottom-left corner, so coordinates are converted
//! on the way out.

use printpdf::path::PaintMode;
use printpdf::*;
use std::io::Cursor;
use std::sync::OnceLock;

/// Bundled Noto Sans (SIL OFL 1.1) — embedded so generated PDFs are portable
/// and render with consistent, properly-spaced glyphs instead of the built-in
/// base-14 fonts.
static FONT_REGULAR: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
static FONT_BOLD: &[u8] = include_bytes!("../fonts/NotoSans-Bold.ttf");

/// Lazily-parsed regular face used to measure text advance widths. Bold widths
/// differ only marginally, so the regular face is used for all measurements.
fn metrics_face() -> &'static ttf_parser::Face<'static> {
    static FACE: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| ttf_parser::Face::parse(FONT_REGULAR, 0).expect("valid bundled font"))
}

/// A single available item shown on the menu.
pub struct MenuItem {
    pub name: String,
    pub price: f64,
    /// Optional description, shown under the name for main-course items.
    pub description: Option<String>,
    /// Absolute/relative filesystem path to the item image, if any
    /// (e.g. `data/item_images/<uuid>.webp`). `None` means no picture.
    pub image_path: Option<String>,
}

/// A category and its available items.
pub struct MenuSection {
    pub name: String,
    pub main_course: bool,
    pub items: Vec<MenuItem>,
}

// A4 portrait, all units in millimetres.
const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 18.0;
const CONTENT_W: f32 = PAGE_W - 2.0 * MARGIN;
/// Largest `y` (distance from top) at which content may still be drawn.
const BOTTOM_LIMIT: f32 = PAGE_H - MARGIN;

// Two-column layout: the content area below the header is split into two equal
// columns separated by a gutter, with a vertical divider drawn down its middle.
const GUTTER: f32 = 12.0;
const COL_W: f32 = (CONTENT_W - GUTTER) / 2.0;
const COL_LEFT_X: f32 = MARGIN;
const COL_RIGHT_X: f32 = MARGIN + COL_W + GUTTER;
const COL_SEP_X: f32 = MARGIN + COL_W + GUTTER / 2.0;

/// Side length of the square thumbnail used for main-course items.
const THUMB: f32 = 22.0;

/// Points to millimetres (1pt = 1/72 inch).
const PT_TO_MM: f32 = 25.4 / 72.0;

fn black() -> Color {
    Color::Rgb(Rgb::new(0.1, 0.1, 0.1, None))
}
fn gray() -> Color {
    Color::Rgb(Rgb::new(0.55, 0.55, 0.55, None))
}
/// Section-header band gradient endpoints (R, G, B): dark on the left (where
/// the title sits) fading to a brighter slate on the right.
const BAND_DARK: (f32, f32, f32) = (0.09, 0.10, 0.13);
const BAND_BRIGHT: (f32, f32, f32) = (0.52, 0.55, 0.62);
/// Text colour on the section-header bands.
fn band_text() -> Color {
    Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None))
}

/// Loads an image from disk and returns straight RGB8 bytes plus its pixel
/// dimensions. Any alpha channel is composited over white so the embedded
/// (alpha-less) RGB image still looks right. Returns `None` if the file is
/// missing or can't be decoded.
fn load_image_rgb(path: &str) -> Option<(Vec<u8>, u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    // Auto-detects the format from the magic bytes (png / jpeg / webp).
    // Absolute path because `use printpdf::*` brings printpdf's own `image`
    // module into scope, which would otherwise shadow the `image` crate.
    let img = ::image::load_from_memory(&bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((w as usize) * (h as usize) * 3);
    for px in rgba.pixels() {
        let [r, g, b, a] = px.0;
        let a = a as f32 / 255.0;
        let blend = |c: u8| (c as f32 * a + 255.0 * (1.0 - a)).round() as u8;
        rgb.push(blend(r));
        rgb.push(blend(g));
        rgb.push(blend(b));
    }
    Some((rgb, w, h))
}

/// Truncates `s` with a trailing ellipsis so it fits within `max_width` mm at
/// the given font size; returns it unchanged when it already fits. Prevents a
/// long name from running into the right-aligned price in a narrow column.
fn ellipsize(s: &str, size_pt: f32, max_width: f32) -> String {
    if Pdf::text_width(s, size_pt) <= max_width {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate = format!("{}…", chars.iter().collect::<String>().trim_end());
        if Pdf::text_width(&candidate, size_pt) <= max_width {
            return candidate;
        }
    }
    "…".to_string()
}

struct Pdf {
    doc: PdfDocumentReference,
    layer: PdfLayerReference,
    font: IndirectFontRef,
    font_bold: IndirectFontRef,
    /// Distance from the top of the current page to the drawing cursor, in mm.
    y: f32,
}

impl Pdf {
    /// Width of a string in mm, summed from the real glyph advances of the
    /// bundled font so centring, right-alignment and leader dots line up.
    fn text_width(s: &str, size_pt: f32) -> f32 {
        let face = metrics_face();
        let upm = face.units_per_em() as f32;
        let units: f32 = s
            .chars()
            .map(|ch| {
                face.glyph_index(ch)
                    .and_then(|g| face.glyph_hor_advance(g))
                    .unwrap_or(0) as f32
            })
            .sum();
        units / upm * size_pt * PT_TO_MM
    }

    /// Greedily wraps `s` into lines no wider than `max_width` mm (using the
    /// same width estimate as `text_width`). A word longer than `max_width` is
    /// left on its own line rather than split.
    fn wrap_text(s: &str, size_pt: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in s.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if !current.is_empty() && Pdf::text_width(&candidate, size_pt) > max_width {
                lines.push(std::mem::replace(&mut current, word.to_string()));
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        lines
    }

    fn new_page(&mut self) {
        let (page, layer) = self.doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
        self.layer = self.doc.get_page(page).get_layer(layer);
        self.y = MARGIN;
    }

    /// Starts a new page if `needed` mm of vertical space won't fit below the
    /// cursor on the current page.
    fn ensure(&mut self, needed: f32) {
        if self.y + needed > BOTTOM_LIMIT {
            self.new_page();
        }
    }

    /// Draws a line of text with its visual top at `y_top` mm from the page top.
    fn text(&self, s: &str, size_pt: f32, x: f32, y_top: f32, bold: bool, color: Color) {
        let font = if bold { &self.font_bold } else { &self.font };
        // Approximate cap height above the baseline (~0.7em).
        let ascent = size_pt * 0.7 * PT_TO_MM;
        let baseline_from_top = y_top + ascent;
        self.layer.set_fill_color(color);
        self.layer
            .use_text(s, size_pt, Mm(x), Mm(PAGE_H - baseline_from_top), font);
    }

    /// Draws a section-header band spanning the full content width at `y_top`
    /// with the given `height`, filled with a left-to-right dark→bright
    /// gradient, and the title (white, bold) on the dark end. printpdf 0.7 has
    /// no native gradient, so it is approximated with thin vertical strips.
    fn section_band(&self, title: &str, y_top: f32, height: f32, title_size: f32) {
        const STRIPS: usize = 96;
        let x0 = MARGIN;
        let x1 = PAGE_W - MARGIN;
        let strip_w = (x1 - x0) / STRIPS as f32;
        let (dark, bright) = (BAND_DARK, BAND_BRIGHT);
        for i in 0..STRIPS {
            let t = i as f32 / (STRIPS - 1) as f32;
            let lerp = |a: f32, b: f32| a + (b - a) * t;
            self.layer.set_fill_color(Color::Rgb(Rgb::new(
                lerp(dark.0, bright.0),
                lerp(dark.1, bright.1),
                lerp(dark.2, bright.2),
                None,
            )));
            let sx = x0 + i as f32 * strip_w;
            // Tiny overlap so anti-aliasing leaves no hairline seams.
            let rect = Rect::new(
                Mm(sx),
                Mm(PAGE_H - (y_top + height)),
                Mm(sx + strip_w + 0.15),
                Mm(PAGE_H - y_top),
            )
            .with_mode(PaintMode::Fill);
            self.layer.add_rect(rect);
        }

        let text_top = y_top + (height - title_size * PT_TO_MM) / 2.0;
        self.text(title, title_size, MARGIN + 3.0, text_top, true, band_text());
    }

    /// Draws a `name … price` row inside the column whose left edge is `x_left`
    /// and right edge is `right_edge`, with a run of leader dots filling the gap
    /// between the (possibly truncated) name and the right-aligned price.
    fn name_price_dots(
        &self,
        name: &str,
        price: &str,
        x_left: f32,
        right_edge: f32,
        y_top: f32,
        size: f32,
        bold: bool,
    ) {
        let pw = Pdf::text_width(price, size);
        self.text(price, size, right_edge - pw, y_top, false, black());

        let max_name_w = (right_edge - x_left) - pw - 4.0;
        let name = ellipsize(name, size, max_name_w.max(6.0));
        self.text(&name, size, x_left, y_top, bold, black());

        // Leader dots between the name and the price.
        let name_end = x_left + Pdf::text_width(&name, size) + 1.5;
        let dots_end = right_edge - pw - 1.5;
        let dot_w = Pdf::text_width(".", size).max(0.1);
        let count = ((dots_end - name_end) / dot_w).floor();
        if count >= 1.0 {
            let dots: String = std::iter::repeat('.').take(count as usize).collect();
            self.text(&dots, size, name_end, y_top, false, gray());
        }
    }

    /// Draws a vertical rule at `x` mm spanning the given `y_top` range.
    fn vline(&self, x: f32, y_top_start: f32, y_top_end: f32) {
        self.layer.set_outline_color(gray());
        self.layer.set_outline_thickness(0.4);
        let line = Line {
            points: vec![
                (Point::new(Mm(x), Mm(PAGE_H - y_top_start)), false),
                (Point::new(Mm(x), Mm(PAGE_H - y_top_end)), false),
            ],
            is_closed: false,
        };
        self.layer.add_line(line);
    }

    /// Draws an already-decoded RGB image fitted into the `box_w` x `box_h` box
    /// whose top-left corner is at (`x`, `y_top`) mm, preserving aspect ratio
    /// and centring it within the box.
    fn image(&self, rgb: Vec<u8>, px_w: u32, px_h: u32, x: f32, y_top: f32, box_w: f32, box_h: f32) {
        let aspect = px_w as f32 / px_h as f32;
        let (mut w, mut h) = (box_w, box_w / aspect);
        if h > box_h {
            h = box_h;
            w = box_h * aspect;
        }
        let off_x = x + (box_w - w) / 2.0;
        let off_y_top = y_top + (box_h - h) / 2.0;
        // dpi chosen so that px_w renders to exactly `w` mm.
        let dpi = px_w as f32 * 25.4 / w;

        let xobject = ImageXObject {
            width: Px(px_w as usize),
            height: Px(px_h as usize),
            color_space: ColorSpace::Rgb,
            bits_per_component: ColorBits::Bit8,
            interpolate: false,
            image_data: rgb,
            image_filter: None,
            smask: None,
            clipping_bbox: None,
        };
        Image::from(xobject).add_to_layer(
            self.layer.clone(),
            ImageTransform {
                translate_x: Some(Mm(off_x)),
                // printpdf anchors the image by its bottom-left corner.
                translate_y: Some(Mm(PAGE_H - off_y_top - h)),
                rotate: None,
                scale_x: None,
                scale_y: None,
                dpi: Some(dpi),
            },
        );
    }
}

// Font sizes (pt) and per-section spacing for the item lists.
const MAIN_NAME_SIZE: f32 = 12.0;
const MAIN_DESC_SIZE: f32 = 8.5;
const TEXT_LINE_SIZE: f32 = 10.5;
/// Vertical gap below each row of items, in mm.
const ROW_GAP: f32 = 4.0;
/// Gap after a section before the next, in mm.
const SECTION_GAP: f32 = 7.0;

/// A measured item ready to draw. Heights are computed up front (descriptions
/// wrapped, thumbnail decoded) so rows can be laid out and paginated before
/// anything is committed to the page.
struct Prepared {
    name: String,
    price: String,
    main: bool,
    thumb: Option<(Vec<u8>, u32, u32)>,
    desc_lines: Vec<String>,
    /// Total height of the item within its column, in mm.
    height: f32,
}

/// Height in mm of a main-course item's name + (optional) description block.
fn main_text_block_h(desc_lines: usize) -> f32 {
    let name_h = MAIN_NAME_SIZE * PT_TO_MM;
    let desc_line_h = MAIN_DESC_SIZE * PT_TO_MM + 1.2;
    name_h
        + if desc_lines == 0 {
            0.0
        } else {
            2.0 + desc_lines as f32 * desc_line_h
        }
}

impl Prepared {
    fn new(item: &MenuItem, main: bool, price_str: &impl Fn(f64) -> String) -> Self {
        let price = price_str(item.price);
        if main {
            let thumb = item.image_path.as_deref().and_then(load_image_rgb);
            let indent = if thumb.is_some() { THUMB + 5.0 } else { 0.0 };
            let desc_lines: Vec<String> = item
                .description
                .as_deref()
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(|d| Pdf::wrap_text(d, MAIN_DESC_SIZE, COL_W - indent))
                .unwrap_or_default();
            let height = THUMB.max(main_text_block_h(desc_lines.len()));
            Self { name: item.name.clone(), price, main, thumb, desc_lines, height }
        } else {
            Self {
                name: item.name.clone(),
                price,
                main,
                thumb: None,
                desc_lines: Vec::new(),
                height: TEXT_LINE_SIZE * PT_TO_MM + 2.5,
            }
        }
    }
}

/// Draws a single prepared item into the column whose left edge is `col_x`,
/// with its top at `top` mm.
fn draw_prepared(pdf: &Pdf, p: &Prepared, col_x: f32, top: f32) {
    let right_edge = col_x + COL_W;
    if p.main {
        let indent = if p.thumb.is_some() { THUMB + 5.0 } else { 0.0 };
        let text_x = col_x + indent;
        let block_h = main_text_block_h(p.desc_lines.len());

        if let Some((rgb, w, h)) = &p.thumb {
            pdf.image(rgb.clone(), *w, *h, col_x, top, THUMB, THUMB);
        }

        // Centre the name + description block against the (taller) thumbnail.
        let block_top = top + (p.height - block_h) / 2.0;
        pdf.name_price_dots(&p.name, &p.price, text_x, right_edge, block_top, MAIN_NAME_SIZE, true);

        let mut dy = block_top + MAIN_NAME_SIZE * PT_TO_MM + 2.0;
        for line in &p.desc_lines {
            pdf.text(line, MAIN_DESC_SIZE, text_x, dy, false, gray());
            dy += MAIN_DESC_SIZE * PT_TO_MM + 1.2;
        }
    } else {
        pdf.name_price_dots(&p.name, &p.price, col_x, right_edge, top, TEXT_LINE_SIZE, false);
    }
}

/// Renders a category: a full-width header band followed by its items in a
/// two-column list (row-major) with a vertical divider between the columns.
/// Handles page breaks within the list, re-drawing the divider per page.
fn render_section(pdf: &mut Pdf, name: &str, items: &[Prepared]) {
    if items.is_empty() {
        return;
    }
    let title_size = 13.0;
    let band_h = title_size * PT_TO_MM + 5.0;

    // Row 0 pairs items 0 and 1; keep the band attached to its first row.
    let first_row_h = items[0].height.max(items.get(1).map(|p| p.height).unwrap_or(0.0));
    pdf.ensure(band_h + 3.0 + first_row_h);

    pdf.section_band(name, pdf.y, band_h, title_size);
    pdf.y += band_h + 3.0;

    // A divider is only meaningful once a row actually uses both columns.
    let two_cols = items.len() >= 2;
    let mut seg_top = pdf.y;

    let mut i = 0;
    while i < items.len() {
        let left = &items[i];
        let right = items.get(i + 1);
        let row_h = left.height.max(right.map(|p| p.height).unwrap_or(0.0));

        if pdf.y + row_h > BOTTOM_LIMIT {
            if two_cols {
                pdf.vline(COL_SEP_X, seg_top, pdf.y - ROW_GAP);
            }
            pdf.new_page();
            seg_top = pdf.y;
        }

        let top = pdf.y;
        draw_prepared(pdf, left, COL_LEFT_X, top);
        if let Some(r) = right {
            draw_prepared(pdf, r, COL_RIGHT_X, top);
        }
        pdf.y += row_h + ROW_GAP;
        i += 2;
    }

    if two_cols {
        pdf.vline(COL_SEP_X, seg_top, pdf.y - ROW_GAP);
    }
    pdf.y += SECTION_GAP;
}

/// Builds the menu PDF and returns the raw bytes.
pub fn build_menu_pdf(
    title: &str,
    currency: &str,
    logo_path: &str,
    sections: &[MenuSection],
) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) = PdfDocument::new(title, Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
    // Embed the bundled Noto Sans, subsetted to the glyphs actually used so the
    // PDF stays small.
    let font = doc
        .add_external_font_with_subsetting(Cursor::new(FONT_REGULAR), true)
        .map_err(|e| e.to_string())?;
    let font_bold = doc
        .add_external_font_with_subsetting(Cursor::new(FONT_BOLD), true)
        .map_err(|e| e.to_string())?;
    let layer = doc.get_page(page1).get_layer(layer1);

    let mut pdf = Pdf {
        doc,
        layer,
        font,
        font_bold,
        y: MARGIN,
    };

    let price_str = |price: f64| format!("{} {:.2}", currency, price);

    // --- Header: logo + title (full width) ---
    if let Some((rgb, w, h)) = load_image_rgb(logo_path) {
        let box_w = 85.0_f32.min(CONTENT_W);
        let box_h = 24.0;
        let drawn_h = (box_w / (w as f32 / h as f32)).min(box_h);
        let x = MARGIN + (CONTENT_W - box_w) / 2.0;
        pdf.image(rgb, w, h, x, pdf.y, box_w, box_h);
        pdf.y += drawn_h + 6.0;
    }
    {
        let size = 26.0;
        let tw = Pdf::text_width(title, size);
        let x = (MARGIN + (CONTENT_W - tw) / 2.0).max(MARGIN);
        pdf.text(title, size, x, pdf.y, true, black());
        pdf.y += size * PT_TO_MM * 1.2 + 8.0;
    }

    // Every non-empty category (main courses first) is rendered as a full-width
    // header band followed by a two-column list of its items.
    let nonempty = |s: &&MenuSection| !s.items.is_empty();
    let ordered = sections
        .iter()
        .filter(nonempty)
        .filter(|s| s.main_course)
        .chain(sections.iter().filter(nonempty).filter(|s| !s.main_course));

    for section in ordered {
        let prepared: Vec<Prepared> = section
            .items
            .iter()
            .map(|it| Prepared::new(it, section.main_course, &price_str))
            .collect();
        render_section(&mut pdf, &section.name, &prepared);
    }

    pdf.doc.save_to_bytes().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_valid_pdf() {
        let sections = vec![
            MenuSection {
                name: "Burgers".into(),
                main_course: true,
                items: vec![
                    MenuItem {
                        name: "Cheeseburger".into(),
                        price: 9.5,
                        description: Some(
                            "Beef patty, cheddar, lettuce, tomato and our house sauce on a brioche bun"
                                .into(),
                        ),
                        image_path: None,
                    },
                    MenuItem { name: "Veggie Burger".into(), price: 8.0, description: None, image_path: None },
                ],
            },
            MenuSection {
                name: "Drinks".into(),
                main_course: false,
                items: vec![MenuItem { name: "Cola".into(), price: 2.5, description: None, image_path: None }],
            },
        ];
        // Use a logo path that does not exist to exercise the missing-image path.
        let bytes = build_menu_pdf("Menu", "€", "does/not/exist.png", &sections).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "output should be a PDF");
        assert!(bytes.len() > 500, "PDF should have real content");
    }

    #[test]
    fn paginates_long_menus() {
        // Enough items to overflow a single page and force `new_page()`.
        let items: Vec<MenuItem> = (0..120)
            .map(|i| MenuItem { name: format!("Item {i}"), price: i as f64, description: None, image_path: None })
            .collect();
        let sections = vec![MenuSection { name: "Many".into(), main_course: false, items }];
        let bytes = build_menu_pdf("Menu", "$", "does/not/exist.png", &sections).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }
}



