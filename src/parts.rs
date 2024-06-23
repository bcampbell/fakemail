use std::collections::HashMap;
use std::fmt;
use std::str;
//use textnonce::TextNonce;
use encoding::{Encoding, DecoderTrap};
use encoding::all::ASCII;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use base64::{display::Base64Display, engine::general_purpose::STANDARD};
use std::path::Path;
use std::fs::File;
use std::io::Read;

// Helpers for building up multipart mime parts.
// Can either use the create_* functions, or just
// construct Part with whatever headers and body you want.
//
// Part has a Display() implementation, which looks at the headers
// for body encoding (Content-Transfer-Encoding), and boundary
// (from Content-Type, for multipart).

#[derive(Clone)]
pub enum Body {
    Data(Vec<u8>),          // For non-multipart types.
    Children(Vec<Part>),    // For multipart types.
}

// Represents a single mime part, which could contain children.
#[derive(Clone)]
pub struct Part {
    pub headers: HashMap<String, String>,
    pub body: Body,
}

// Create a plain text mime part. txt assumed to be 7bit safe.
pub fn create_plaintext(txt: &str) -> Part {
    let hdrs: HashMap<String, String> = HashMap::new();
    // Assume text/plain as default.
    // hdrs.insert("Content-Type".to_string(), "text/plain".to_string());
    Part{headers: hdrs, body: Body::Data(txt.as_bytes().to_vec())}
}

// Create a generic data chunk.
pub fn create_data(data: Vec<u8>, mime_type: &str) -> Part {
    let mut hdrs: HashMap<String, String> = HashMap::new();
    hdrs.insert("Content-Type".to_string(), mime_type.to_string());
    hdrs.insert("Content-Transfer-Encoding".to_string(), "base64".to_string());
    Part{headers: hdrs, body: Body::Data(data)}
}

// Create a multipart/mixed part, with the given children.
pub fn create_multipart_mixed(children: Vec<Part>) -> Part {
    let mut hdrs: HashMap<String, String> = HashMap::new();

    let ct = format!("multipart/mixed; boundary=\"{}\"", random_string(42));
    hdrs.insert("Content-Type".to_string(), ct);
    Part{headers: hdrs, body: Body::Children(children)}
}

// Create an attachment part by loading in a file.
// Will guess content type based on filename.
// TODO: create_attachment() should probably be failable (i/o can fail).
// For now, just panics.
pub fn create_attachment(file_path: &Path) -> Part {
    let mut hdrs: HashMap<String, String> = HashMap::new();

    // Pick a MIME media type based on file extension.
    let extension = match file_path.extension() {
        Some(x) => x.to_str().unwrap(),
        None => "",
    };
    let media_type = match extension.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        _ => "application/octet-stream",
    };
    hdrs.insert("Content-Type".to_string(), media_type.to_string());
    hdrs.insert("Content-Transfer-Encoding".to_string(), "base64".to_string());
    hdrs.insert("Content-Disposition".to_string(), format!("attachment; filename=\"{}\";", file_path.file_name().unwrap().to_str().unwrap()));

    let mut f = File::open(file_path).unwrap();
    let mut data = vec![];
    f.read_to_end(&mut data).unwrap();

    Part{headers: hdrs, body: Body::Data(data)}
}




/*
boundary := 0*69<bchars> bcharsnospace
bchars := bcharsnospace / " "
bcharsnospace :=    DIGIT / ALPHA / "'" / "(" / ")" / "+"  / "_"
                           / "," / "-" / "." / "/" / ":" / "=" / "?"
*/

// Generate a string we can use for multipart boundaries.
fn random_string(n: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric)
                .take(n)
                .map(char::from)  // From link above, this is needed in later versions
                .collect()
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Output headers
        for (name, val) in &self.headers {
            write!(f, "{}: {}\r\n", name, val)?;
        }
        write!(f, "\r\n")?;


        match &self.body {
            Body::Children(children) => {
                // Get the boundary string from Content-Type.
                // TODO: ensure content type is a multipart type.
                let content_type = match self.headers.get("Content-Type") {
                    Some(x) => x,
                    None => panic!(),   // TODO: something better.
                };
                let mime = content_type.parse::<mime::Mime>().unwrap();
                let bound = mime.get_param(mime::BOUNDARY).unwrap();

                for c in children {
                    write!(f, "\r\n--{}\r\n", bound)?;
                    write!(f, "{}", c)?;
                }
                write!(f, "\r\n--{}--\r\n", bound)?;
            },
            Body::Data(data) => {
                // Get content-transfer-encoding (assume 7bit if missing).
                let enc = match self.headers.get("Content-Transfer-Encoding") {
                    Some(x) => x.to_lowercase(),
                    None => "".to_string(),
                };
                match enc.as_str() {
                    "7bit" | "" => {
                        let s = ASCII.decode(data, DecoderTrap::Strict).unwrap();
                        write!(f, "{}", s)?;
                    },
                    "base64" => {

                        let wrapper = Base64Display::new(data, &STANDARD);
                        write!(f, "{}", wrapper)?;
                    },
                    _ => {
                        panic!();
                    },
                }


            },
        }
        Ok(())
    }
}


