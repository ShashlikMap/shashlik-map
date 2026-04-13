use nmea_parser::{NmeaParser, ParsedMessage};
use serial2::SerialPort;
use std::io::{BufRead, BufReader};
use std::thread::spawn;


pub fn start_sgnss<F>(callback: F)
where
    F: Fn(f64, f64) + Send + Sync + 'static,
{
    spawn(move || {
        let mut parser = NmeaParser::new();
        let port = SerialPort::open("/dev/serial0", 9600).unwrap();

        let mut reader = BufReader::new(port);
        let mut buffer = String::new();
        loop {
            match reader.read_line(&mut buffer) {
                Ok(bytes_read) if bytes_read > 0 => {
                    // print!("Received: {}", buffer);
                    let parsed = parser.parse_sentence(buffer.as_str());
                    if let Ok(parsed) = parsed {
                        match parsed {
                            ParsedMessage::Gll(gll) => {
                                println!("NMEA: LatLon: {:?}, {:?}", gll.latitude, gll.longitude);
                                if let (Some(lat), Some(lon)) = (gll.latitude, gll.longitude) {
                                    callback(lat, lon);
                                }
                            }
                            _ => {}
                        }
                    } else {
                        eprintln!("Bad NMEA data: {:?}", parsed);
                    }
                    buffer.clear(); // Clear the buffer for the next line
                }
                Ok(_) => break, // EOF reached
                Err(e) => eprintln!("Error reading: {:?}", e),
            }
        }
    });
}
