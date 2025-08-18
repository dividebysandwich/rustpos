use glob::glob;
use recibo::{Printer, FileDriver, Alignment};

// Try to open a port with recibo and send a basic init.
// Returns Ok(printer) if successful, Err otherwise.
fn try_printer_on_port(path: &str) -> Result<Printer, Box<dyn std::error::Error>> {
    let driver = FileDriver::new(path)?;
    let mut printer = Printer::open(driver)?;
    // Probe with init + feed. If this fails, it's likely not a printer.
    printer.init()?;//.feed(1)?;
    Ok(printer)
}

// Scan common serial device paths and return the first usable printer.
pub fn find_printer() -> Result<(String, Printer), Box<dyn std::error::Error>> {
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
                // Print debug
                println!("Trying on port: {}", path_str);
                if let Ok(printer) = try_printer_on_port(&path_str) {
                    return Ok((path_str, printer));
                }
            }
        }
    }

    Err("No ESC/POS printer found on serial ports".into())
}

pub fn print_receipt(printer: &mut Printer, items: Vec<(String, u32, f32)>, paid_amount: f32, change: f32) -> Result<(), Box<dyn std::error::Error>> {
    printer.init()?;
    printer.align(Alignment::Center)?;
    printer.linespacing(1)?;
    printer.text("RECEIPT\n")?;
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
    printer.text(&format!("Paid: {:.2}\n", paid_amount))?;
    printer.text(&format!("Change: {:.2}\n", change))?;
    printer.feed(6)?;
    printer.cut()?;
    Ok(())
}