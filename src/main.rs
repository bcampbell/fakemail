use std::collections::HashMap;

use chrono::prelude::*;
use fake::faker::chrono::raw::DateTime;
use fake::faker::chrono::raw::DateTimeAfter;
use fake::faker::chrono::raw::DateTimeBetween;
use fake::faker::company::raw::*;
use fake::faker::internet::raw::*;
use fake::faker::lorem::en::*;
use fake::locales::*;
use fake::Fake;
use rand::Rng; // 0.8.0
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use clap::Parser;

/// Generate fake emails for testing.
/// Can produce either a single mbox file, or multiple .eml files.
#[derive(Parser, Debug)]
#[clap(name = "fakemail")]
#[clap(version = "0.1", author = "ben@scumways.com")]
struct Args {
    /// Output format (mbox, eml)
    #[clap(short, default_value = "mbox")]
    format: String,

    /// Output file for mbox, dir for eml (defaults: stdout/cwd).
    #[clap(short)]
    output: Option<String>,

    /// Number of emails to generate
    #[clap(short, default_value = "1")]
    num: u32,
}

fn init_output(args: &Args) -> Box<dyn Dumper> {
    if args.format == "eml" {
        Box::new(EMLDumper::new(args))
    } else {
        Box::new(MBoxDumper::new(&args.output))
    }
}

fn main() {
    let args = Args::parse();

    // Maintain a stack of replies.
    let mut stack: Vec<Email> = Vec::new();
    let mut out = init_output(&args);

    let mut count = 0;
    loop {
        if count >= args.num {
            break;
        }
        // A couple of chances to go back up the thread...
        let choice = rand::thread_rng().gen_range(0..100);
        if choice < 50 {
            stack.pop();
        }
        if choice < 80 {
            stack.pop();
        }
        let e = generate(stack.last());
        out.dump(&e).expect("dump failed!");
        stack.push(e);

        count = count + 1;
    }
}

struct Email {
    headers: HashMap<String, String>,
    body: String,
}

// Create a single email.
// If the parent is set, the generated email will be a reply.
fn generate(parent: Option<&Email>) -> Email {
    let mut hdrs: HashMap<String, String> = HashMap::new();

    // headers, from rfc2822:
    //
    // "Date"
    // "From"
    // "To"
    // "Subject"
    // "Message-ID"

    // "In-Reply-To"
    // "References"
    let from: String = SafeEmail(EN).fake();
    hdrs.insert("From".to_string(), from);

    let ns = Utc::now().timestamp_nanos() as i64;
    let domain: String = FreeEmailProvider(EN).fake();

    let message_id = format!("<{}@{}>", ns % 1000000, domain);
    hdrs.insert("Message-ID".to_string(), String::from(&message_id));

    // Crafting a reply?
    if let Some(m) = &parent {
        let parent_id = m.headers.get("Message-ID").unwrap();
        hdrs.insert(String::from("In-Reply-To"), parent_id.to_string());

        let mut refs: String;
        if m.headers.contains_key("References") {
            refs = m.headers.get("References").unwrap().to_string();
            refs.push(' ');
        } else if m.headers.contains_key("In-Reply-To") {
            refs = m.headers.get("In-Reply-To").unwrap().to_string();
            refs.push(' ');
        } else {
            refs = String::from("");
        }

        refs += &parent_id;

        hdrs.insert(String::from("References"), refs.to_string());

        // reuse subject
        let subj = m.headers.get("Subject").unwrap();
        if subj.starts_with("Re: ") {
            hdrs.insert(String::from("Subject"), subj.to_string());
        } else {
            let new_subj = &format!("Re: {}", subj).to_string();
            hdrs.insert(String::from("Subject"), new_subj.to_string());
        }

        // If parent has a date, make the reply come later.
        let parent_date_hdr = m.headers.get("Date");
        let date: chrono::DateTime<Utc> = match parent_date_hdr {
            Some(hdr) => {
                let parent_date: chrono::DateTime<Utc> = chrono::DateTime::parse_from_rfc2822(hdr)
                    .expect("Couldn't parse date in header")
                    .into();
                DateTimeAfter(EN, parent_date).fake()
            }
            None => DateTime(EN).fake(),
        };
        hdrs.insert(String::from("Date"), date.to_rfc2822());
    } else {
        let date: chrono::DateTime<Utc> = DateTimeBetween(
            EN,
            Utc.with_ymd_and_hms(1990, 1, 1, 0, 0, 0).unwrap(),
            Utc::now(),
        )
        .fake();
        hdrs.insert(String::from("Date"), date.to_rfc2822());
        // a brand new subject
        let subj = CatchPhase(EN).fake();
        hdrs.insert(String::from("Subject"), subj);
    }

    let words: Vec<String> = Sentences(1..10).fake();

    Email {
        headers: hdrs,
        body: words.join("\r\n"),
    }
}

trait Dumper {
    fn dump(&mut self, email: &Email) -> std::io::Result<()>;
}

struct MBoxDumper<'a> {
    out: Box<dyn std::io::Write + 'a>,
}

impl<'a> MBoxDumper<'a> {
    fn new(outfile: &Option<String>) -> MBoxDumper<'a> {
        let f: Box<dyn std::io::Write> = match outfile {
            Some(name) => Box::new(std::fs::File::create(name).expect("Couldn't create mbox")),
            None => Box::new(std::io::stdout()),
        };
        MBoxDumper { out: f }
    }
}

impl Dumper for MBoxDumper<'_> {
    fn dump(&mut self, email: &Email) -> std::io::Result<()> {
        write!(self.out, "From \r\n")?;
        for (name, val) in &email.headers {
            write!(self.out, "{}: {}\r\n", name, val)?;
        }
        write!(self.out, "\r\n{}\r\n", email.body)?;
        write!(self.out, "\r\n")?;
        Ok(())
    }
}

struct EMLDumper {
    outdir: String,
    i: u32,
}

impl EMLDumper {
    fn new(args: &Args) -> EMLDumper {
        let f: String = match &args.output {
            Some(name) => name.clone(),
            None => ".".to_string(),
        };
        EMLDumper { outdir: f, i: 0 }
    }
}

impl Dumper for EMLDumper {
    fn dump(&mut self, email: &Email) -> std::io::Result<()> {
        let mut path = PathBuf::from(self.outdir.to_string());
        path.push(format!("{}.eml", self.i));
        self.i = self.i + 1;

        let mut f = File::create(path)?;
        for (name, val) in &email.headers {
            write!(f, "{}: {}\r\n", name, val)?;
        }
        write!(f, "\r\n{}\r\n", email.body)?;
        write!(f, "\r\n")?;

        Ok(())
    }
}
