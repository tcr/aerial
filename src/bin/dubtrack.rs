extern crate aerial;
extern crate html5ever;
extern crate hyper;
extern crate nix;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tendril;
extern crate url;

use aerial::{fetch_string, youtube, soundcloud, snitch};
use nix::sys::signal::{kill, SIGSTOP, SIGCONT};
use serde_json::Value;
use std::env;
use std::io::{BufReader, BufRead, Read, Write};
use std::io;
use std::process::{Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver};
use std::thread;
use regex::Regex;
use std::time::Duration;

fn main () {
    let mut last: String = "".into();

    loop {
        let body = fetch_string(&format!("http://api.dubtrack.fm/room/{}", env::args().nth(1).unwrap()));
        let test: Value = serde_json::from_str(&body).unwrap();

        if test.lookup("data.currentSong.fkid").is_none() {
            writeln!(io::stderr(), "no song playing right now: {:?}", test.lookup("data.currentSong"));
        } else {
            let id = test.lookup("data.currentSong.fkid").unwrap().as_string().unwrap();
            let songtype = test.lookup("data.currentSong.type").unwrap().as_string().unwrap();

            let cur = format!("{}:{}", songtype, id);
            if cur != last {
                println!("{}", cur);
                last = cur;
            }
        }

        thread::sleep(Duration::from_secs(30));
    }
}
