//! Server-side generation of a printable PDF menu sheet.
//!
//! Renders the restaurant logo and a menu title, followed by the items of every
//! category flagged as a "main course" (each shown with its image and price),
//! and finally the remaining categories as compact, image-less sections.
//!
//! Layout is done by hand on an A4 portrait page. The logo and title span the
//! full width; everything below flows in two equal columns separated by a
//! vertical divider. A top-down cursor (`y` is the distance from the top edge
//! in millimetres) fills the left column, then the right, then a new page.
//! printpdf places everything from the bottom-left corner, so coordinates are
//! converted on the way out.

use printpdf::*;

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
    /// Current column: 0 = left, 1 = right.
    col: usize,
    /// Left edge of the current column's content, in mm.
    col_x: f32,
    /// `y` at which the columns begin (below the header on page 1, the top
    /// margin on later pages). Used to reset the cursor on a column break.
    content_top: f32,
}

impl Pdf {
    /// Rough width of a string in mm. Helvetica glyphs average ~0.5em; this is
    /// only used for centring/right-aligning, so an approximation is fine.
    fn text_width(s: &str, size_pt: f32) -> f32 {
        s.chars().count() as f32 * size_pt * 0.5 * PT_TO_MM
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
        // Later pages have no header, so columns start at the top margin.
        self.begin_columns(MARGIN);
    }

    /// Begins the two-column area at `content_top`: resets to the left column
    /// and draws the vertical divider down the gutter for the current page.
    fn begin_columns(&mut self, content_top: f32) {
        self.content_top = content_top;
        self.col = 0;
        self.col_x = COL_LEFT_X;
        self.y = content_top;
        self.vline(COL_SEP_X, content_top, BOTTOM_LIMIT);
    }

    /// Moves the cursor to the next column, or to a fresh page when the right
    /// column is exhausted.
    fn next_column(&mut self) {
        if self.col == 0 {
            self.col = 1;
            self.col_x = COL_RIGHT_X;
            self.y = self.content_top;
        } else {
            self.new_page();
        }
    }

    /// Breaks to the next column if `needed` mm of vertical space won't fit
    /// below the cursor in the current column.
    fn ensure(&mut self, needed: f32) {
        if self.y + needed > BOTTOM_LIMIT {
            self.next_column();
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

    /// Draws a horizontal rule across the current column at `y_top` mm.
    fn hrule(&self, y_top: f32) {
        self.layer.set_outline_color(gray());
        self.layer.set_outline_thickness(0.4);
        let yb = PAGE_H - y_top;
        let line = Line {
            points: vec![
                (Point::new(Mm(self.col_x), Mm(yb)), false),
                (Point::new(Mm(self.col_x + COL_W), Mm(yb)), false),
            ],
            is_closed: false,
        };
        self.layer.add_line(line);
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

/// Builds the menu PDF and returns the raw bytes.
pub fn build_menu_pdf(
    title: &str,
    currency: &str,
    logo_path: &str,
    sections: &[MenuSection],
) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) = PdfDocument::new(title, Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;
    let layer = doc.get_page(page1).get_layer(layer1);

    let mut pdf = Pdf {
        doc,
        layer,
        font,
        font_bold,
        y: MARGIN,
        col: 0,
        col_x: COL_LEFT_X,
        content_top: MARGIN,
    };

    let price_str = |price: f64| format!("{} {:.2}", currency, price);

    // --- Header: logo + title ---
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

    // Everything below the header flows in two columns split by a divider.
    let content_top = pdf.y;
    pdf.begin_columns(content_top);

    // --- Main course sections (with images) ---
    for section in sections.iter().filter(|s| s.main_course && !s.items.is_empty()) {
        // Keep the header with at least its first item (avoids an orphan header
        // at the foot of a column).
        pdf.ensure(THUMB + 18.0);
        let header_size = 15.0;
        pdf.text(&section.name, header_size, pdf.col_x, pdf.y, true, black());
        pdf.y += header_size * PT_TO_MM + 2.0;
        pdf.hrule(pdf.y);
        pdf.y += 4.0;

        for item in &section.items {
            let name_size = 12.0;
            let desc_size = 8.5;

            // Decode the thumbnail up front so the text indent and row height
            // can account for it.
            let thumb = item.image_path.as_deref().and_then(load_image_rgb);
            let indent = if thumb.is_some() { THUMB + 5.0 } else { 0.0 };

            // Wrap the description to the width left in the column.
            let desc_width = COL_W - indent;
            let desc_lines: Vec<String> = item
                .description
                .as_deref()
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(|d| Pdf::wrap_text(d, desc_size, desc_width))
                .unwrap_or_default();

            let name_h = name_size * PT_TO_MM;
            let desc_line_h = desc_size * PT_TO_MM + 1.2;
            let text_block_h = name_h
                + if desc_lines.is_empty() {
                    0.0
                } else {
                    2.0 + desc_lines.len() as f32 * desc_line_h
                };
            let row_h = THUMB.max(text_block_h);

            pdf.ensure(row_h + 4.0);
            let row_top = pdf.y;
            let text_x = pdf.col_x + indent;
            let right_edge = pdf.col_x + COL_W;

            if let Some((rgb, w, h)) = thumb {
                pdf.image(rgb, w, h, pdf.col_x, row_top, THUMB, THUMB);
            }

            // Vertically centre the name + description block against the row.
            let block_top = row_top + (row_h - text_block_h) / 2.0;

            // Right-align the price, then wrap the name to the space left of it.
            let p = price_str(item.price);
            let pw = Pdf::text_width(&p, name_size);
            pdf.text(&p, name_size, right_edge - pw, block_top, false, black());

            let name_width = (right_edge - text_x) - pw - 2.0;
            let name = ellipsize(&item.name, name_size, name_width.max(8.0));
            pdf.text(&name, name_size, text_x, block_top, true, black());

            // Description below the name, in a smaller, lighter font.
            let mut dy = block_top + name_h + 2.0;
            for line in &desc_lines {
                pdf.text(line, desc_size, text_x, dy, false, gray());
                dy += desc_line_h;
            }

            pdf.y = row_top + row_h + 4.0;
        }
        pdf.y += 6.0;
    }

    // --- Remaining sections (text only) ---
    for section in sections.iter().filter(|s| !s.main_course && !s.items.is_empty()) {
        pdf.ensure(18.0);
        let header_size = 13.0;
        pdf.text(&section.name, header_size, pdf.col_x, pdf.y, true, black());
        pdf.y += header_size * PT_TO_MM + 2.0;
        pdf.hrule(pdf.y);
        pdf.y += 3.5;

        for item in &section.items {
            let line_size = 10.5;
            let line_h = line_size * PT_TO_MM + 2.5;
            pdf.ensure(line_h);
            let right_edge = pdf.col_x + COL_W;

            let p = price_str(item.price);
            let pw = Pdf::text_width(&p, line_size);
            pdf.text(&p, line_size, right_edge - pw, pdf.y, false, black());

            let name_width = COL_W - pw - 2.0;
            let name = ellipsize(&item.name, line_size, name_width.max(8.0));
            pdf.text(&name, line_size, pdf.col_x, pdf.y, false, black());

            pdf.y += line_h;
        }
        pdf.y += 6.0;
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

