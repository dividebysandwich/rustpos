use std::sync::atomic::{AtomicU8, Ordering};

use chrono::{DateTime, Local};
use encoding::EncoderTrap;
use glob::glob;
use recibo::{Alignment, Encoder, GraphicSize, Printer, FileDriver};

// ESC/POS "select character code table" page for Windows-1252 (WPC1252).
// Thermal printers don't understand UTF-8: they map each byte through a
// single-byte code page. We encode text as Windows-1252 (which contains the
// German umlauts ä ö ü Ä Ö Ü ß) and tell the printer to interpret the bytes
// the same way, otherwise multi-byte UTF-8 sequences print as garbage glyphs.
//
// 16 (WPC1252) is the default and works for the tested Munbyn ITPP098 and most
// Epson-compatible printers. It can be overridden at startup via set_codepage()
// for printers that number their code pages differently.
pub const DEFAULT_CODEPAGE: u8 = 16;

static CODEPAGE: AtomicU8 = AtomicU8::new(DEFAULT_CODEPAGE);

/// Override the ESC/POS code page sent to the printer. Call once at startup
/// (before printing) from whatever configuration source applies. A value of 0
/// is treated as "unset" and leaves the default in place.
pub fn set_codepage(page: u8) {
    if page != 0 {
        CODEPAGE.store(page, Ordering::Relaxed);
    }
}

/// The ESC/POS code page currently in effect.
pub fn codepage() -> u8 {
    CODEPAGE.load(Ordering::Relaxed)
}

/// Open (kick) the cash drawer wired to the receipt printer's RJ11/RJ12
/// drawer-kick port via the ESC/POS "generate pulse" command (ESC p m t1 t2).
///
/// m = 0 selects drawer-kick connector pin 2 (the common wiring); t1/t2 set the
/// pulse on/off time in units of 2 ms, so 0x19/0x78 = 50 ms on / 240 ms off,
/// enough to fire the usual 12 V/24 V drawer solenoid. If a drawer is wired to
/// pin 5 instead, change m to 1.
pub fn open_cash_drawer(printer: &mut Printer) -> Result<(), Box<dyn std::error::Error>> {
    printer.write(&[0x1B, 0x70, 0x00, 0x19, 0x78])?;
    Ok(())
}

fn try_printer_on_port(path: &str) -> Result<Printer, Box<dyn std::error::Error>> {
    let driver = FileDriver::new(path)?;
    let encoder = Encoder::new(encoding::all::WINDOWS_1252, EncoderTrap::Replace);
    let mut printer = Printer::builder().driver(driver).encoder(encoder).build();
    printer.init()?;
    select_codepage(&mut printer)?;
    Ok(printer)
}

// ESC t n — select the character code table on the printer. Must be re-sent
// after every init() (ESC @), which resets the printer to its power-on code page.
fn select_codepage(printer: &mut Printer) -> Result<(), Box<dyn std::error::Error>> {
    printer.write(&[0x1B, 0x74, codepage()])?;
    Ok(())
}

pub fn find_printer() -> Result<(String, Printer), Box<dyn std::error::Error>> {
    // Try Linux-style device paths first
    #[cfg(not(target_os = "windows"))]
    {
        let candidates = vec![
            "/dev/ttyUSB*",
            "/dev/ttyACM*",
            "/dev/usb/lp*",
            "/dev/serial/by-id/*",
        ];

        for pattern in candidates {
            for entry in glob(pattern)? {
                if let Ok(path) = entry {
                    let path_str = path.display().to_string();
                    println!("Trying on port: {}", path_str);
                    if let Ok(printer) = try_printer_on_port(&path_str) {
                        return Ok((path_str, printer));
                    }
                }
            }
        }
    }

    // Try serial port enumeration (primary method on Windows, fallback on Linux)
    if let Ok(ports) = serialport::available_ports() {
        for port in ports {
            let path = if cfg!(windows) {
                format!("\\\\.\\{}", port.port_name)
            } else {
                port.port_name.clone()
            };
            println!("Trying on port: {}", port.port_name);
            if let Ok(printer) = try_printer_on_port(&path) {
                return Ok((port.port_name, printer));
            }
        }
    }

    Err("No ESC/POS printer found on serial ports".into())
}

pub fn print_credentials(
    printer: &mut Printer,
    username: &str,
    pin: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    printer.init()?;
    select_codepage(printer)?;
    printer.align(Alignment::Center)?;
    printer.bold(true)?;
    printer.text("================================\n")?;
    printer.text("  RustPOS Initial Setup\n")?;
    printer.text("================================\n")?;
    printer.bold(false)?;
    printer.feed(1)?;
    printer.align(Alignment::Left)?;
    printer.text(&format!("  Username: {}\n", username))?;
    printer.text(&format!("  PIN:      {}\n", pin))?;
    printer.feed(1)?;
    printer.align(Alignment::Center)?;
    printer.text("Please change your PIN\n")?;
    printer.text("after first login.\n")?;
    printer.feed(4)?;
    printer.cut()?;
    Ok(())
}

/// Truncate to `width` characters (char-safe) and left-pad to that width.
fn fit_left(text: &str, width: usize) -> String {
    let s: String = text.chars().take(width).collect();
    format!("{:<width$}", s, width = width)
}

/// Print a sales breakdown for a period: per-item quantity sold and total value.
pub fn print_sales_report(
    printer: &mut Printer,
    period: &str,
    currency: &str,
    items: Vec<(String, u32, f32)>,
    total_items_sold: u32,
    total_revenue: f32,
    datetime: DateTime<Local>,
    logo_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    printer.init()?;
    select_codepage(printer)?;
    // Open the cash drawer when the sales report is printed (end-of-day cash count).
    open_cash_drawer(printer)?;
    printer.align(Alignment::Center)?;
    printer.linespacing(1)?;
    if let Some(logo) = logo_path {
        let logo_owned = logo.to_string();
        printer.graphic(move |builder| {
            builder.path(&logo_owned).size(GraphicSize::Normal)
        })?;
    }
    printer.bold(true)?;
    printer.text("Sales Report\n")?;
    printer.bold(false)?;
    printer.text(&format!("{}\n", period))?;
    printer.text("------------------------------------------------\n")?;

    printer.align(Alignment::Left)?;
    // Columns: name (26) | qty (6) | total (16) = 48 chars
    printer.text(&format!("{}{:>6}{:>16}\n", fit_left("Item", 26), "Qty", "Total"))?;
    printer.text("------------------------------------------------\n")?;
    for (name, qty, revenue) in &items {
        let line = format!("{}{:>6}{:>16.2}\n", fit_left(name, 26), qty, revenue);
        printer.text(&line)?;
    }

    printer.align(Alignment::Center)?;
    printer.text("------------------------------------------------\n")?;
    printer.align(Alignment::Left)?;
    printer.bold(true)?;
    printer.text(&format!("Items sold: {}\n", total_items_sold))?;
    printer.text(&format!("TOTAL {}: {:.2}\n", currency, total_revenue))?;
    printer.bold(false)?;
    printer.feed(1)?;
    printer.text(&format!(
        "Printed: {}\n",
        datetime.format("%Y-%m-%d %H:%M:%S")
    ))?;
    printer.feed(6)?;
    printer.cut()?;
    Ok(())
}

pub fn print_receipt(
    printer: &mut Printer,
    items: Vec<(String, u32, f32)>,
    paid_amount: f32,
    change: f32,
    datetime: DateTime<Local>,
    logo_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    printer.init()?;
    select_codepage(printer)?;
    // Open the cash drawer on every sale.
    open_cash_drawer(printer)?;
    printer.align(Alignment::Center)?;
    printer.linespacing(1)?;
    if let Some(logo) = logo_path {
        let logo_owned = logo.to_string();
        printer.graphic(move |builder| {
            builder
                .path(&logo_owned)
                .size(GraphicSize::Normal)
        })?;
    }
    printer.text("------------------------------------------------\n")?;

    printer.align(Alignment::Left)?;
    let mut total = 0.0;
    for (name, qty, price) in &items {
        let line = format!("{:<20} {:>2} x {:>18.2}\n", name, qty, price);
        printer.text(&line)?;
        total += (*qty as f32) * price;
    }

    printer.align(Alignment::Center)?;
    printer.text("------------------------------------------------\n")?;
    printer.align(Alignment::Left)?;
    printer.bold(true)?;
    printer.text(&format!("TOTAL: {:>35.2}\n", total))?;
    printer.text("------------------------------------------------\n")?;
    printer.feed(1)?;
    printer.bold(false)?;
    printer.text(&format!(
        "Date: {}\n",
        datetime.format("%Y-%m-%d %H:%M:%S")
    ))?;
    printer.text(&format!("Cash: {:.2}\n", paid_amount))?;
    printer.text(&format!("Change: {:.2}\n", change))?;
    printer.feed(1)?;
    printer.align(Alignment::Center)?;
    printer.qr(|builder| {
        builder.size(200).text(&format!(
            "{}|Total:{}|Given:{}|Change:{}",
            datetime.format("%Y-%m-%d %H:%M:%S"),
            total,
            paid_amount,
            change
        ))
    })?;
    printer.feed(6)?;
    printer.cut()?;
    Ok(())
}
