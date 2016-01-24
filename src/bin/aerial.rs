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

fn main () {    
    // let keys = Command::new("/Users/timryan/tcr/aerial/mediakeys/osx/out/Default/keylistener")
    //     .stdin(Stdio::piped())
    //     .stdout(Stdio::piped())
    //     .stderr(Stdio::piped())
    //     .spawn()
    //     .unwrap();

    let (tx, rx) = sync_channel(0);
    // thread::spawn(move || {
    //     let keys_out = keys.stdout.unwrap();
    //     for line in BufReader::new(keys_out).lines() {
    //         tx.send(line.unwrap()).unwrap();
    //     }
    // });

    loop {
        // let body = fetch_string(&format!("http://api.dubtrack.fm/room/{}", env::args().nth(1).unwrap()));
        // let test: Value = serde_json::from_str(&body).unwrap();

        // if test.lookup("data.currentSong.fkid").is_none() {
        //     println!("No song playing right now: {:?}", test.lookup("data.currentSong"));
        //     break;
        // }

        // println!("Song: {:?}", test.lookup("data.currentSong.name").unwrap());

        // let id = test.lookup("data.currentSong.fkid").unwrap().as_string().unwrap();
        // let songtype = test.lookup("data.currentSong.type").unwrap().as_string().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                let mut items = input.split(":");
                let songtype = items.next().unwrap();
                let id = items.next().unwrap();

                // println!("Type: {:?}", songtype);
                let song = if songtype == "soundcloud" {
                    soundcloud::fetch(id).unwrap()
                } else {
                    youtube::fetch(id).unwrap()
                };

                play_interactive(Box::new(song), &rx);

                println!("");
                println!("");
            }
            Err(error) => {
                println!("input terminated: {:?}", error);
                break;
            }
        }
    }
}

#[allow(boxed_local)]
fn play_interactive(mut stream: Box<Read + Send>, keys: &Receiver<String>) {
    let ffmpeg = Command::new("ffmpeg").args(&["-i", "-", "-f", "mp3", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let play = Command::new("play").args(&["-t", "mp3", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();


    let playpid = play.id() as i32;
    let mut ffmpeg_stdin = ffmpeg.stdin.unwrap();
    let mut ffmpeg_stdout = ffmpeg.stdout.unwrap();
    let mut play_stdin = play.stdin.unwrap();
    let mut play_stderr = play.stderr.unwrap();

    let t1 = thread::spawn(move || {
        io::copy(&mut stream, &mut ffmpeg_stdin).ok();
    });

    let t2 = thread::spawn(move || {
        io::copy(&mut ffmpeg_stdout, &mut play_stdin).ok();
    });

    let t3 = thread::spawn(move || {
        for item in snitch(&mut play_stderr, &Regex::new(r"In:.*?\].*?\]").unwrap()) {
            write!(&mut io::stderr(), "{}\r", item.snitch().at(0).unwrap());
        }
    });

    // let mut playing = true;
    // loop {
    //     let key = match keys.recv() {
    //         Ok(value) => {
    //             value
    //         }
    //         err => {
    //             println!("error: {:?}", err);
    //             break;
    //         }
    //     };
    //     if key == "play" {
    //         if playing {
    //             kill(playpid, SIGSTOP).ok();
    //         } else {
    //             kill(playpid, SIGCONT).ok();
    //         }
    //         playing = !playing;
    //     }
    // }

    t1.join().ok();
    t2.join().ok();
    t3.join().ok();
}
