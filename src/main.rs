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
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;
//use rand::prelude::IteratorRandom;
use rand::prelude::SliceRandom;
use clap::Parser;

mod parts;

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

    /// Directory holding files to randomly attach to messages.
    #[clap(short)]
    attach_dir: Option<String>,

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

// Pick a random file from dir_path (which can be missing).
pub fn pick_file(dir_path: Option<&str>) -> Option<PathBuf> {
    match dir_path {
        Some(dir) => {
            // from https://stackoverflow.com/questions/58062887/filtering-files-or-directories-discovered-with-fsread-dir
            let files: Vec<_> = fs::read_dir(dir).unwrap()
                .into_iter()
                .filter(|r| r.is_ok()) // Get rid of Err variants for Result<DirEntry>
                .map(|r| r.unwrap().path()) // This is safe, since we only have the Ok variants
                .filter(|r| r.is_file()) // Filter out dirs
                .collect();

            let mut rng = rand::thread_rng();
            match files.choose(&mut rng) {
                Some(x) => Some(x.to_path_buf()),
                None => None,
            }
        },
        None => None,
    }
}

fn main() {
    let args = Args::parse();

    // Maintain a stack of replies.
    // Each part here is the root mimepart of the email.
    let mut stack: Vec<parts::Part> = Vec::new();
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
        let e = generate(stack.last(), &args.attach_dir);
        out.dump(&e).expect("dump failed!");
        stack.push(e);

        count = count + 1;
    }
}

// Create a single email.
// If the parent is set, the generated email will be a reply.
// Returns the root mimepart of the email. Might be the only part,
// or might be multipart...
fn generate(parent: Option<&parts::Part>, attach_dir: &Option<String>) -> parts::Part {

    // Start off by generating some headers for the email.
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

        // Reuse subject.
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
        let subj = CatchPhrase(EN).fake();
        hdrs.insert(String::from("Subject"), subj);
    }


    // Generate email body.
    let words: Vec<String> = Sentences(1..10).fake();
    let text = words.join("\r\n") + "\r\n";
    let plain = parts::create_plaintext(&text);

    // If there's an attachment dir set, randomly pick 0..4 files to
    // attach to the message.
    let mut rng = rand::thread_rng();
    let num_attachments = match attach_dir {
        Some(_d) => [0,0,0,0,0,0,0,1,1,1,1,2,2,2,3,3,4].choose(&mut rng).unwrap().clone(),
        None => 0,
    };


    let mut attachments : Vec<parts::Part> = Vec::new();
    for _ in 0..num_attachments {
        match pick_file(attach_dir.as_deref()) {
            Some(f) =>{
                let att = parts::create_attachment(&f);
                attachments.push(att);
            },
            None => {},
        };
    }


    let mut root = if attachments.len() > 0 {
        let mut children : Vec<parts::Part> = Vec::new();
        children.push(plain);   // Use our text/plain as the first part.
        children.append(&mut attachments);
        parts::create_multipart_mixed(children)
    } else {
        plain               // Single part, text-only message.
    };

    // Add the main email headers (To/From/Subject etc) to the root part.
    root.headers.extend(hdrs);
    root
}

trait Dumper {
    fn dump(&mut self, email: &parts::Part) -> std::io::Result<()>;
}

// For writing emails out to a single mbox file.
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
    fn dump(&mut self, email: &parts::Part) -> std::io::Result<()> {
        // TODO: escape "From " lines in body!
        write!(self.out, "From \r\n")?;
        write!(self.out, "{}\r\n", email)?;
        Ok(())
    }
}

// For writing out emails as individual files to outdir.
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
    fn dump(&mut self, email: &parts::Part) -> std::io::Result<()> {
        let mut path = PathBuf::from(self.outdir.to_string());
        path.push(format!("{}.eml", self.i));
        self.i = self.i + 1;

        let mut f = File::create(path)?;
        write!(f, "{}", email)?;

        Ok(())
    }
}

