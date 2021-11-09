//use std::io;
//use fake::{Dummy, Fake, Faker};
//use rand::rngs::StdRng;
//use rand::SeedableRng;
extern crate chrono;
use std::collections::HashMap;

use chrono::prelude::*;
use fake::faker::chrono::raw::DateTime;
use fake::faker::company::raw::*;
use fake::faker::internet::raw::*;
use fake::faker::lorem::en::*;
use fake::locales::*;
use fake::Fake;
use rand::Rng; // 0.8.0
//use std::io::Write;
//use std::fs::File;


use clap::Clap;

/// Generate fake emails for testing.
/// Outputs in mbox format.
#[derive(Clap, Debug)]
#[clap(name = "fakemail")]
#[clap(version = "0.1", author = "ben@scumways.com")]
struct Args {
    /// Output format
    #[clap(short, default_value = "mbox")]
    format: String,

    /// Output location (file for mbox, dir for .eml)
    #[clap(short)] 
    output: Option<String>,

    /// Number of emails to generate
    #[clap(short, default_value = "1")]
    num: u32,
}


fn init_output(args: &Args) -> Box<dyn Dumper> {

    if args.format=="eml" {
        Box::new(EMLDumper{})
    } else {
        Box::new(MBoxDumper::new(&args.output))
    }
}

fn main() {
    let args = Args::parse();

    let mut stack: Vec<Email> = Vec::new();
    let mut out = init_output(&args);

    //let out = &mut Foo{};

    let mut count = 0;
    loop {
        if count >= args.num {
            break;
        }
        let choice = rand::thread_rng().gen_range(0..100);
        if choice < 50 {
            stack.pop();
        }
        if choice < 80 {
            stack.pop();
        }

        let e = generate(stack.last());
        out.dump(&e);
        stack.push(e);

        count = count + 1;
    }
}

struct Email {
    headers: HashMap<String, String>,
    body: String,
}

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
    // "References" ?
    let from: String = SafeEmail(EN).fake();
    hdrs.insert("From".to_string(), from);

    let ns = Utc::now().timestamp_nanos() as i64;
    hdrs.insert(String::from("Message-ID"), format!("<{}>", ns));

    // Crafting a reply?
    if let Some(m) = &parent {
        let parent_id = m.headers.get("Message-ID").unwrap();
        hdrs.insert(String::from("In-Reply-To"), parent_id.to_string());
        hdrs.insert(String::from("References"), parent_id.to_string());

        // reuse subject
        let subj = m.headers.get("Subject").unwrap();
        if subj.starts_with("Re: ") {
            hdrs.insert(String::from("Subject"), subj.to_string());
        } else {
            let new_subj = &format!("Re: {}", subj).to_string();
            hdrs.insert(String::from("Subject"), new_subj.to_string());
        }

        // TODO: parse parent date and move forwards.
        let date: chrono::DateTime<Utc> = DateTime(EN).fake();
        hdrs.insert(String::from("Date"), date.to_rfc2822());
    } else {
        let date: chrono::DateTime<Utc> = DateTime(EN).fake();
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
    fn dump(&mut self, email: &Email);
}


struct MBoxDumper<'a> {
    out: Box<dyn std::io::Write + 'a>
}
        
impl<'a> MBoxDumper<'a> {
    fn new(outfile: &Option<String>) -> MBoxDumper<'a> {
        let f: Box<dyn std::io::Write> = match outfile {
            Some(name) => Box::new(std::fs::File::create(name).expect("Couldn't create mbox")),
            None => Box::new(std::io::stdout()),
        };
        MBoxDumper{ out: f }
    }
}

impl Dumper for MBoxDumper<'_> {
    fn dump(&mut self, email: &Email) {
        write!(self.out, "From \r\n").expect("write fail");
        for (name, val) in &email.headers {
            write!(self.out, "{}: {}\r\n", name, val).expect("write fail");
        }
        write!(self.out, "\r\n{}\r\n", email.body).expect("write fail");
        write!(self.out, "\r\n").expect("write fail");
    }
}

struct EMLDumper {
}

impl Dumper for EMLDumper {
    fn dump(&mut self, _email: &Email) {
        println!("EMAIL!");
    }
}

