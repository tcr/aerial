extern crate html5ever;
extern crate hyper;
extern crate nix;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tendril;
extern crate url;

pub mod youtube;
pub mod soundcloud;

use html5ever::rcdom::{Text, Element, Handle};
use hyper::Client;
use hyper::header::Connection;
use regex::{Captures, Regex};
use std::io::{BufReader, BufRead, Read, Cursor, ErrorKind};
use serde_json::error::Error as DeError;
use serde_json::error::ErrorCode as DeErrorCode;
use serde_json::Value;

fn get_scripts(handle: Handle) -> Vec<String> {
    let mut res = vec![];
    let node = handle.borrow();

    if let Element(ref name, _, _) = node.node {
        if &*name.local == "script" {
            for child in &node.children {
                if let Text(ref text) = child.borrow().node {
                    res.push(text.to_string());
                }
            }
        }
    }

    for child in &node.children {
        res.extend(get_scripts(child.clone()));
    }

    res
}

pub fn fetch_string(url: &str) -> String {
    let client = Client::new();
    let mut res = client.get(url)
        .header(Connection::close())
        .send().unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).ok();

    body
}

pub fn fetch_json(url: &str) -> serde_json::Result<Value> {
    serde_json::from_str(&fetch_string(url))
}

pub struct Snitch<'t, T: 't + Read> {
    re: &'t Regex,
    reader: BufReader<&'t mut T>,
    buf: Vec<u8>,
}

pub struct SnitchResult<'t> {
    value: String,
    re: &'t Regex,
}

impl<'t> SnitchResult<'t> {
    pub fn snitch(&self) -> Captures {
        self.re.captures(&self.value).unwrap()
    }
}

impl<'t, T: Read> Iterator for Snitch<'t, T> {
    type Item = SnitchResult<'t>;

    fn next<'j>(&'j mut self) -> Option<SnitchResult<'t>> {
        loop {
            let buflen = {
                let input = self.reader.fill_buf();

                let buf = match input {
                    Ok(buf) => {
                        buf
                    },
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(..) => return None,
                };

                if buf.len() == 0 {
                    return None;
                }

                self.buf.extend(buf);
                buf.len()
            };
            self.reader.consume(buflen);

            // TODO trim back buffer
            let value = String::from_utf8_lossy(&self.buf).into_owned();

            if let Some(cap) = self.re.captures(&value) {
                self.buf = self.buf.split_off(value[..cap.pos(0).unwrap().1].as_bytes().len());
            } else {
                continue;
            }
            
            return Some(SnitchResult {
                re: self.re,
                value: value,
            });
        }
    }
}

pub fn snitch<'t, T: Read>(input: &'t mut T, re: &'t Regex) -> Snitch<'t, T> {
    Snitch {
        re: re,
        reader: BufReader::new(input),
        buf: vec![],
    }
}

#[test]
fn test_snitch () {
    let input = "test\nthis\ncool\nthing";
    let mut cur = Cursor::new(input);
    for word in snitch(&mut cur, &Regex::new(r"thi[sng]+").unwrap()) {
        println!("word: {:?}", word.snitch().at(0));
    }
}
