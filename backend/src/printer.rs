use glob::glob;
use recibo::{Printer, FileDriver};

// Try to open a port with recibo and send a basic init.
// Returns Ok(printer) if successful, Err otherwise.
fn try_printer_on_port(path: &str) -> Result<Printer, Box<dyn std::error::Error>> {
    let driver = FileDriver::new(path)?;
    let mut printer = Printer::open(driver)?;
    // Probe with init + feed. If this fails, it's likely not a printer.
    printer.init()?.feed(1)?;
    Ok(printer)
}

// Scan common serial device paths and return the first usable printer.
pub fn find_printer() -> Result<(String, Printer), Box<dyn std::error::Error>> {
    let candidates = vec!["/dev/ttyUSB*", "/dev/ttyACM*", "/dev/serial/by-id/*"];

    for pattern in candidates {
        for entry in glob(pattern)? {
            if let Ok(path) = entry {
                let path_str = path.display().to_string();
                if let Ok(printer) = try_printer_on_port(&path_str) {
                    return Ok((path_str, printer));
                }
            }
        }
    }

    Err("No ESC/POS printer found on serial ports".into())
}

